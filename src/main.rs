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

const WIDTH: u32 = 400;
const HEIGHT: u32 = 400;

fn get_window_size() -> LogicalSize<f64> {
    return LogicalSize::new(WIDTH as f64, HEIGHT as f64);
}

fn get_window_size_scaled() -> LogicalSize<f64> {
    let mut ls = get_window_size();
    ls.width = ls.width * 3.0;
    ls.height = ls.height * 3.0;

    return ls;
}

fn main() -> Result<(), Error> {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = get_window_size();
        let scaled_size = get_window_size_scaled();
        WindowBuilder::new()
            .with_title("hello world!")
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

    let mut life = Grid::new_empty_grid(WIDTH as usize, HEIGHT as usize);
    life.randomize();

    let mut paused = false;
    let mut draw_state: Option<bool> = None;

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            life.draw(pixels.get_frame_mut());

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
            // Keyboard events
            //

            // [ESCAPE]     = Quit
            //
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // [P]          = Toggle Pause
            //
            if input.key_pressed(VirtualKeyCode::P) {
                paused = !paused;
            }

            // [SPACE]      = Pause (for frame step)
            if input.key_pressed_os(VirtualKeyCode::Space) {
                paused = true;
            }

            // [R]          = Randomize TGOL
            if input.key_pressed(VirtualKeyCode::R) {
                life.randomize();
            }

            // Mouse events
            //
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

                debug!("Draw at {:?} => {:?}", mouse_prev_cell, mouse_cell);
                debug!("Mouse held {:?}, release {:?}", held, release);

                // If they either released (finishing the drawing) or are still
                // in the middle of drawing, keep going.
                if release || held {
                    debug!("Draw line of {:?}", draw_alive);
                    life.set_line(
                        mouse_prev_cell.0,
                        mouse_prev_cell.1,
                        mouse_cell.0,
                        mouse_cell.1,
                        draw_alive,
                    );

                    life.draw(pixels.get_frame_mut());
                }

                // If they let go or are otherwise not clicking anymore, stop drawing.
                if release || !held {
                    debug!("Draw end");
                    draw_state = None;
                }
            }

            // Resize the window
            //
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }

            if !paused || input.key_pressed_os(VirtualKeyCode::Space) {
                //life.update();
                life.draw(pixels.get_frame_mut());
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
    //
    alive: bool,

    // Heat: Trailing effect of the cell. Decays over time.
    //
    heat: u8,
}

impl Cell {
    // Initialize a new cell (alive or dead)
    fn new(alive: bool) -> Self {
        Self {
            alive: alive,
            heat: 0,
        }
    }

    fn set(&mut self, alive: bool) {
        self.alive = alive;
    }

    fn toggle(&mut self) -> bool {
        self.alive = !self.alive;
        self.alive
    }
}

const CELL_ALIVE_THRESHOLD: f32 = 0.3;
const GREEN: [u8; 4] = [0, 255, 0, 255];
//const RED: [u8; 4] = [255, 0, 0, 255];
//const BLUE: [u8; 4] = [0, 0, 255, 255];
//const YELLOW: [u8; 4] = [255, 255, 0, 255];

struct Grid {
    grid: Vec<Cell>,
    width: usize,
    height: usize,
}

impl Grid {
    fn new_empty_grid(width: usize, height: usize) -> Self {
        assert!(width != 0);
        assert!(height != 0);

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
        // TODO: Smooth out the noise from randomness
        // TODO: Once we smooth out the randomness get rid of the leftover heatmap
    }

    fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.grid.len());

        for (cell, pix) in self.grid.iter().zip(screen.chunks_exact_mut(4)) {
            let color = if cell.alive {
                GREEN
            } else {
                [0, 0, cell.heat, 0xff]
            };

            pix.copy_from_slice(&color);
        }
    }

    fn toggle(&mut self, x: isize, y: isize) -> bool {
        if let Some(i) = self.grid_idx(x, y) {
            self.grid[i].toggle()
        } else {
            false
        }
    }

    fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, alive: bool) {
        // probably should do sutherland-hodgeman if this were more serious.
        // instead just clamp the start pos, and draw until moving towards the
        // end pos takes us out of bounds.
        let x0 = x0.max(0).min(self.width as isize);
        let y0 = y0.max(0).min(self.height as isize);
        for (x, y) in line_drawing::Bresenham::new((x0, y0), (x1, y1)) {
            if let Some(i) = self.grid_idx(x, y) {
                self.grid[i].set(alive);
            } else {
                break;
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
