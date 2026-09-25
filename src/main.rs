//
// [T]HE [G]AME [O]F [L]IFE
//

#![forbid(unsafe_code)]

use std::sync::Arc;

use log::{debug, error};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, MouseButton, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::{Window, WindowId},
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 16 * 24;
const HEIGHT: u32 = 10 * 24;

fn get_window_size() -> LogicalSize<f64> {
    LogicalSize::new(WIDTH as f64, HEIGHT as f64)
}

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    input: WinitInputHelper,
    life: Grid,
    paused: bool,
    draw_state: Option<bool>,
}

impl App {
    fn new() -> Self {
        let mut life = Grid::new_empty_grid(WIDTH as usize, HEIGHT as usize);
        life.randomize();

        Self {
            window: None,
            pixels: None,
            input: WinitInputHelper::new(),
            life,
            paused: false,
            draw_state: None,
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        if !self.paused {
            self.life.update();
            self.life.draw(pixels.frame_mut());
        }

        if let Err(e) = pixels.render() {
            error!("pixels.render() failed: {}", e);
            event_loop.exit();
        }
    }

    fn handle_input(&mut self, event_loop: &ActiveEventLoop) {
        let input = &self.input;

        // ===========================
        // Keyboard events
        // ===========================

        // [ESCAPE]     = Quit
        if input.key_pressed(KeyCode::Escape) || input.close_requested() || input.destroyed() {
            log::info!("Escape pressed. Quitting..");
            event_loop.exit();
            return;
        }

        // [SPACE]      = Pause (for frame step)
        if input.key_pressed_os(KeyCode::Space) {
            log::info!("'SPACE' pressed. Pausing..");
            self.paused = true;
        }

        // [P]          = Toggle Pause
        if input.key_pressed(KeyCode::KeyP) {
            log::info!("'P' pressed. Toggling pause..");
            self.paused = !self.paused;
        }

        // [R]          = Randomize TGOL
        if input.key_pressed(KeyCode::KeyR) {
            log::info!("'R' pressed. Randomizing..");
            self.life.randomize();
        }

        // [K]          = KILL Random cells
        if input.key_pressed(KeyCode::KeyK) {
            let kill_count = self.life.randomly_kill();
            log::info!("'K' pressed. Randomly killed {:?} cells..", kill_count);
        }

        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        // ================================
        // Mouse events
        // ================================
        let (mouse_cell, mouse_prev_cell) = input
            .cursor()
            .map(|(mx, my)| {
                let (dx, dy) = input.cursor_diff();
                let prev_x = mx - dx;
                let prev_y = my - dy;

                let (mx_i, my_i) = pixels
                    .window_pos_to_pixel((mx, my))
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                let (px_i, py_i) = pixels
                    .window_pos_to_pixel((prev_x, prev_y))
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                (
                    (mx_i as isize, my_i as isize),
                    (px_i as isize, py_i as isize),
                )
            })
            .unwrap_or_default();

        if input.mouse_pressed(MouseButton::Left) {
            debug!("Mouse click at {:?}", mouse_cell);
            self.draw_state = Some(self.life.toggle(mouse_cell.0, mouse_cell.1));
        } else if let Some(draw_alive) = self.draw_state {
            let release = input.mouse_released(MouseButton::Left);
            let held = input.mouse_held(MouseButton::Left);

            // If they either released (finishing the drawing) or are still
            // in the middle of drawing, keep going.
            if release || held {
                self.life.set_line(
                    mouse_prev_cell.0,
                    mouse_prev_cell.1,
                    mouse_cell.0,
                    mouse_cell.1,
                    draw_alive,
                );
            }

            // If they let go or are otherwise not clicking anymore, stop drawing.
            if release || !held {
                debug!("Draw end");
                self.draw_state = None;
            }
        }

        // ====================================
        // WINDOW RESIZE events
        // ====================================

        if let Some(size) = input.window_resized() {
            log::info!(
                "Window resize. Width: {:?}, Height: {:?}",
                size.width,
                size.height
            );

            if let Err(e) = pixels.resize_surface(size.width, size.height) {
                error!("pixels.resize_surface() failed: {}", e);
                event_loop.exit();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let size = get_window_size();
        let attributes = Window::default_attributes()
            .with_title(format!("TGOL [{} x {}]", WIDTH, HEIGHT))
            .with_inner_size(size)
            .with_min_inner_size(size);

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                error!("Failed to create window: {}", e);
                event_loop.exit();
                return;
            }
        };

        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());

        match Pixels::new(WIDTH, HEIGHT, surface_texture) {
            Ok(pixels) => self.pixels = Some(pixels),
            Err(e) => {
                error!("Failed to create pixel buffer: {}", e);
                event_loop.exit();
                return;
            }
        }

        window.request_redraw();
        self.window = Some(window);
    }

    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        self.input.step();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.input.process_window_event(&event) {
            self.render(event_loop);
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        self.input.process_device_event(&event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.input.end_step();
        self.handle_input(event_loop);

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::new())?;

    Ok(())
}

/// Generate a pseudorandom seed for the game's PRNG.
fn generate_seed() -> (u64, u64) {
    use byteorder::{ByteOrder, NativeEndian};
    use getrandom::getrandom;

    let mut seed = [0_u8; 16];

    getrandom(&mut seed).expect("failed to getrandom");

    (
        NativeEndian::read_u64(&seed[0..8]),
        NativeEndian::read_u64(&seed[8..16]),
    )
}

#[derive(Clone, Copy, Debug, Default)]
struct Cell {
    // Alive: Is this cell active or not
    alive: bool,

    // Heat: Trailing effect of the cell. Decays over time.
    heat: u8,
}

impl Cell {
    // Initialize a new cell (alive or dead)
    fn new(alive: bool) -> Self {
        let heat = if alive { 255 } else { 0 };
        Self {
            alive: alive,
            heat: heat,
        }
    }

    // cools off a cell, returns T if the cell was alive
    // but has died. Otherwise false.
    fn cool_if_dead(&mut self, subtract_count: u8) {
        if !self.alive && self.heat > 0 {
            self.heat = self.heat.saturating_sub(subtract_count);
        }
    }

    fn set(&mut self, alive: bool) {
        self.alive = alive;

        if self.alive {
            self.heat = 255;
        }
    }
}

const CELL_ALIVE_THRESHOLD: f32 = 0.3;

struct Grid {
    grid: Vec<Cell>,
    width: usize,
    height: usize,
}

impl Grid {
    fn update(&mut self) {
        //
        // Allocate a new grid (only swap out after computation has finished.
        // This way we don't get any 'tearing' if we want to extend this routine
        // to be multithreaded. For situations like iterating over a clock.
        //
        let mut grid_tmp = self.grid.clone();

        //
        // Compute, figure out what the next grid frame is going to look like.
        //

        for x in 0..self.width {
            for y in 0..self.height {
                let neighbors_alive = self.count_neighbors(x, y);

                if let Some(cell) = self.grid_idx(x, y) {
                    // RULE #1: Any live cell with two or three live neighbours survives.
                    // RULE #2: Any dead cell with three live neighbours becomes a live cell.
                    // RULE #3: All other live cells die in the next generation. Similarly, all other dead cells stay dead.
                    if self.grid[cell].alive {
                        if neighbors_alive == 2 || neighbors_alive == 3 {
                            grid_tmp[cell].set(true); // RULE # 1
                            continue;
                        }
                    } else {
                        if neighbors_alive == 3 {
                            grid_tmp[cell].set(true); // RULE #2
                            continue;
                        }
                    }

                    grid_tmp[cell].set(false); // RULE #3
                    grid_tmp[cell].cool_if_dead(50);
                } else {
                    assert!(false);
                }
            }
        }

        //
        // SWAP, Compute finished.. swap out to the new graph.
        //
        std::mem::swap(&mut grid_tmp, &mut self.grid);
    }

    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        //
        // final two sets of coords. an (x1, y1)
        // that indicates the coords of the neighboring
        // grid (UP-LEFT) and another set of coords (x2, y2)
        // that represents the coords to the (BOTTOM-RIGHT)
        //

        let (xm1, xp1) = if x == 0 {
            (self.width - 1, x + 1)
        } else if x == self.width - 1 {
            (x - 1, 0)
        } else {
            (x - 1, x + 1)
        };

        let (ym1, yp1) = if y == 0 {
            (self.height - 1, y + 1)
        } else if y == self.height - 1 {
            (y - 1, 0)
        } else {
            (y - 1, y + 1)
        };

        //
        // This is a fancy way to add up all the neighboring
        // cells. If they are alive.
        //
        self.grid[xm1 + ym1 * self.width].alive as usize
            + self.grid[x + ym1 * self.width].alive as usize
            + self.grid[xp1 + ym1 * self.width].alive as usize
            + self.grid[xm1 + y * self.width].alive as usize
            + self.grid[xp1 + y * self.width].alive as usize
            + self.grid[xm1 + yp1 * self.width].alive as usize
            + self.grid[x + yp1 * self.width].alive as usize
            + self.grid[xp1 + yp1 * self.width].alive as usize
    }

    fn new_empty_grid(width: usize, height: usize) -> Self {
        let size = width.checked_mul(height).expect("Grid too big (overflow)");
        Self {
            grid: vec![Cell::default(); size],
            width,
            height,
        }
    }

    fn randomize(&mut self) {
        let mut rand: randomize::PCG32 = generate_seed().into();

        for cell in self.grid.iter_mut() {
            let alive = randomize::f32_half_open_right(rand.next_u32()) > CELL_ALIVE_THRESHOLD;
            *cell = Cell::new(alive);
        }

        self.normalize(5);
    }

    fn randomly_kill(&mut self) -> u32 {
        let mut rand: randomize::PCG32 = generate_seed().into();
        let mut kill_count: u32 = 0;

        for cell in self.grid.iter_mut() {
            if cell.alive {
                let kill = randomize::f32_half_open_right(rand.next_u32()) > CELL_ALIVE_THRESHOLD;
                if kill {
                    cell.set(false);
                    kill_count += 1;
                }
            }
        }

        kill_count
    }

    // const GREEN: [u8; 4] = [0, 255, 0, 255];
    // const RED: [u8; 4] = [255, 0, 0, 255];
    // const BLUE: [u8; 4] = [0, 0, 255, 255];
    // const YELLOW: [u8; 4] = [255, 255, 0, 255];

    fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.grid.len());

        for (cell, pix) in self.grid.iter().zip(screen.chunks_exact_mut(4)) {
            let color = if !cell.alive {
                [
                    cell.heat.saturating_sub(100),
                    0,
                    cell.heat.saturating_sub(30),
                    cell.heat.saturating_sub(30),
                ]
            } else {
                [50, 0, 0xff, 0xff]
            };

            pix.copy_from_slice(&color);
        }
    }

    fn toggle(&mut self, x: isize, y: isize) -> bool {
        if let Some(i) = self.grid_idx(x, y) {
            if self.grid[i].alive {
                self.grid[i].set(false);
                false
            } else {
                self.grid[i].set(true);
                true
            }
        } else {
            false
        }
    }

    fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, alive: bool) {
        let x0 = x0.max(0).min(self.width as isize);
        let y0 = y0.max(0).min(self.height as isize);
        for (x, y) in line_drawing::Bresenham::new((x0, y0), (x1, y1)) {
            if let Some(i) = self.grid_idx(x, y) {
                if self.grid[i].alive != alive {
                    self.grid[i].set(alive);
                }
            } else {
                break;
            }
        }
    }

    fn normalize(&mut self, generations: usize) {
        // Kill of a random amount of the cells. The grid starts too noisy.
        self.randomly_kill();

        // Pass x amount of generations.
        for _ in 0..generations {
            self.update();
        }

        // Now we need to cool off the heatmap that is leftover
        // Otherwise is looks messy.
        for cell in self.grid.iter_mut() {
            if !cell.alive {
                cell.heat = 0;
            }
        }
    }

    fn grid_idx<I: std::convert::TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        if let (Ok(x), Ok(y)) = (x.try_into(), y.try_into()) {
            if x < self.width && y < self.height {
                Some(x + y * self.width)
            } else {
                None
            }
        } else {
            None
        }
    }
}
