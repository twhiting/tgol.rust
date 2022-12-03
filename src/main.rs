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

    // let mut life = ConwayGrid::new_random(WIDTH as usize, HEIGHT as usize);
    let mut paused = false;
    let mut draw_state: Option<bool> = None;

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            // life.draw(pixels.get_frame_mut())
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
                // life.randomize();
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
                //draw_state = Some(life.toggle(mouse_cell.0, mouse_cell.1));
            } else if let Some(draw_alive) = draw_state {
                let release = input.mouse_released(0);
                let held = input.mouse_held(0);

                debug!("Draw at {:?} => {:?}", mouse_prev_cell, mouse_cell);
                debug!("Mouse held {:?}, release {:?}", held, release);

                // If they either released (finishing the drawing) or are still
                // in the middle of drawing, keep going.
                if release || held {
                    debug!("Draw line of {:?}", draw_alive);
                    /* life.set_line(
                        mouse_prev_cell.0,
                        mouse_prev_cell.1,
                        mouse_cell.0,
                        mouse_cell.1,
                        draw_alive,
                    ); */
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
            }

            window.request_redraw();
        }
    });
}
