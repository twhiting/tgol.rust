//
// [T]HE [G]AME [O]F [L]IFE
//

#![forbid(unsafe_code)]

use log::{debug, error};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 16 * 24;
const HEIGHT: u32 = 10 * 24;

fn get_window_size() -> LogicalSize<f64> {
    LogicalSize::new(WIDTH as f64, HEIGHT as f64)
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = get_window_size();

        WindowBuilder::new()
            .with_title(format!("TGOL [{} x {}]", WIDTH, HEIGHT))
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut paused = false;
    let mut draw_state: Option<bool> = None;

    let mut life = Grid::new_empty_grid(WIDTH as usize, HEIGHT as usize);
    life.randomize();

    event_loop.run(move |event, _, control_flow| {
        // log::info!("<loop>");

        if let Event::RedrawRequested(_) = event {
            if !paused {
                life.update();
                life.draw(pixels.get_frame_mut());
            }

            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if input.update(&event) {
            // ===========================
            // Keyboard events
            // ===========================

            // [ESCAPE]     = Quit
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                log::info!("Escape pressed. Quitting..");
                *control_flow = ControlFlow::Exit;
                return;
            }

            // [SPACE]      = Pause (for frame step)
            if input.key_pressed_os(VirtualKeyCode::Space) {
                log::info!("'SPACE' pressed. Pausing..");
                paused = true;
            }

            // [P]          = Toggle Pause
            if input.key_pressed(VirtualKeyCode::P) {
                log::info!("'P' pressed. Toggling pause..");
                paused = !paused;
            }

            // [R]          = Randomize TGOL
            if input.key_pressed(VirtualKeyCode::R) {
                log::info!("'R' pressed. Randomizing..");
                life.randomize();
            }

            // [K]          = KILL Random cells
            if input.key_pressed(VirtualKeyCode::K) {
                let kill_count = life.randomly_kill();
                log::info!("'K' pressed. Randomly killed {:?} cells..", kill_count);
            }

            // ================================
            // Mouse events
            // ================================
            let (mouse_cell, mouse_prev_cell) = input
                .mouse()
                .map(|(mx, my)| {
                    let (dx, dy) = input.mouse_diff();
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

            if input.mouse_pressed(0) {
                debug!("Mouse click at {:?}", mouse_cell);
                draw_state = Some(life.toggle(mouse_cell.0, mouse_cell.1));
            } else if let Some(draw_alive) = draw_state {
                let release = input.mouse_released(0);
                let held = input.mouse_held(0);

                // debug!("Draw at {:?} => {:?}", mouse_prev_cell, mouse_cell);
                // debug!("Mouse held {:?}, release {:?}", held, release);

                // If they either released (finishing the drawing) or are still
                // in the middle of drawing, keep going.
                if release || held {
                    // debug!("Draw line of {:?}", draw_alive);
                    life.set_line(
                        mouse_prev_cell.0,
                        mouse_prev_cell.1,
                        mouse_cell.0,
                        mouse_cell.1,
                        draw_alive,
                    );

                    // life.draw(pixels.get_frame_mut());
                }

                // If they let go or are otherwise not clicking anymore, stop drawing.
                if release || !held {
                    debug!("Draw end");
                    draw_state = None;
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

                pixels.resize_surface(size.width, size.height);
            }

            window.request_redraw();
        }
    });
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
        let size = self
            .width
            .checked_mul(self.height)
            .expect("Grid too big (overflow)");

        // let mut grid_tmp: Vec<Cell> = vec![Cell::default(); size];
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
                if !self.grid[i].alive {
                    self.grid[i].set(true);
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
