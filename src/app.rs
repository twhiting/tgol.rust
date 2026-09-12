use crate::life::Grid;
use log::{debug, error};
use pixels::{Error, Pixels, SurfaceTexture};
use std::time::{Duration, Instant};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 16 * 24;
const HEIGHT: u32 = 10 * 24;

pub(crate) fn run() -> Result<(), Error> {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = window_size();

        WindowBuilder::new()
            .with_title(window_title())
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

    let mut life = Grid::new_empty(WIDTH as usize, HEIGHT as usize);
    life.randomize();
    let mut debug_overlay = DebugOverlay::new();

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            let update_started = debug_overlay.enabled.then(Instant::now);
            let updating = !paused;
            if updating {
                let should_continue = life.update();
                debug_overlay.record_update(
                    update_started
                        .map(|started| started.elapsed())
                        .unwrap_or_default(),
                );
                if !should_continue {
                    paused = true;
                }
            }

            let render_started = debug_overlay.enabled.then(Instant::now);
            if updating {
                life.draw(pixels.frame_mut());
            }

            let render_result = pixels.render();
            debug_overlay.record_render(
                render_started
                    .map(|started| started.elapsed())
                    .unwrap_or_default(),
            );
            debug_overlay.refresh_title(&window, &life, paused, updating && paused);

            if render_result
                .map_err(|error| error!("pixels.render() failed: {}", error))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                log::info!("Escape pressed. Quitting..");
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.key_pressed(VirtualKeyCode::F3) {
                debug_overlay.toggle();
                if debug_overlay.enabled {
                    debug_overlay.refresh_title(&window, &life, paused, true);
                } else {
                    window.set_title(&window_title());
                }
            }

            if input.key_pressed_os(VirtualKeyCode::Space) {
                log::info!("'SPACE' pressed. Pausing..");
                paused = true;
                debug_overlay.refresh_title(&window, &life, paused, true);
            }

            if input.key_pressed(VirtualKeyCode::P) {
                log::info!("'P' pressed. Toggling pause..");
                paused = !paused;
                debug_overlay.refresh_title(&window, &life, paused, true);
            }

            if input.key_pressed(VirtualKeyCode::R) {
                log::info!("'R' pressed. Randomizing..");
                life.randomize();
                debug_overlay.reset_generation();
                debug_overlay.refresh_title(&window, &life, paused, true);
            }

            if input.key_pressed(VirtualKeyCode::K) {
                let kill_count = life.randomly_kill();
                log::info!("'K' pressed. Randomly killed {:?} cells..", kill_count);
            }

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
                if paused {
                    paused = false;
                    debug_overlay.refresh_title(&window, &life, paused, true);
                }
            } else if let Some(draw_alive) = draw_state {
                let release = input.mouse_released(0);
                let held = input.mouse_held(0);

                if release || held {
                    life.set_line(
                        mouse_prev_cell.0,
                        mouse_prev_cell.1,
                        mouse_cell.0,
                        mouse_cell.1,
                        draw_alive,
                    );
                }

                if release || !held {
                    debug!("Draw end");
                    draw_state = None;
                }
            }

            if let Some(size) = input.window_resized() {
                log::info!(
                    "Window resize. Width: {:?}, Height: {:?}",
                    size.width,
                    size.height
                );

                if let Err(error) = pixels.resize_surface(size.width, size.height) {
                    error!("pixels.resize_surface() failed: {}", error);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            window.request_redraw();
        }
    });
}

fn window_size() -> LogicalSize<f64> {
    LogicalSize::new(WIDTH as f64, HEIGHT as f64)
}

fn window_title() -> String {
    format!("TGOL [{} x {}]", WIDTH, HEIGHT)
}

struct DebugOverlay {
    enabled: bool,
    generation: u64,
    sample_started: Instant,
    rendered_frames: u64,
    updates: u64,
    update_time: Duration,
    render_time: Duration,
    fps: f64,
    average_update_ms: f64,
    average_render_ms: f64,
}

impl DebugOverlay {
    fn new() -> Self {
        Self {
            enabled: false,
            generation: 0,
            sample_started: Instant::now(),
            rendered_frames: 0,
            updates: 0,
            update_time: Duration::ZERO,
            render_time: Duration::ZERO,
            fps: 0.0,
            average_update_ms: 0.0,
            average_render_ms: 0.0,
        }
    }

    fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.reset_sample();
    }

    fn reset_sample(&mut self) {
        self.sample_started = Instant::now();
        self.rendered_frames = 0;
        self.updates = 0;
        self.update_time = Duration::ZERO;
        self.render_time = Duration::ZERO;
    }

    fn record_update(&mut self, elapsed: Duration) {
        self.generation = self.generation.saturating_add(1);
        if self.enabled {
            self.updates += 1;
            self.update_time += elapsed;
        }
    }

    fn record_render(&mut self, elapsed: Duration) {
        if self.enabled {
            self.rendered_frames += 1;
            self.render_time += elapsed;
        }
    }

    fn reset_generation(&mut self) {
        self.generation = 0;
    }

    fn refresh_title(&mut self, window: &Window, grid: &Grid, paused: bool, force: bool) {
        if !self.enabled {
            return;
        }

        let sample_elapsed = self.sample_started.elapsed();
        if sample_elapsed >= Duration::from_secs(1) {
            self.fps = self.rendered_frames as f64 / sample_elapsed.as_secs_f64();
            self.average_update_ms = average_milliseconds(self.update_time, self.updates);
            self.average_render_ms = average_milliseconds(self.render_time, self.rendered_frames);
            self.reset_sample();
        } else if !force {
            return;
        }

        let alive = grid.alive_count();
        let population = alive as f64 * 100.0 / grid.cell_count() as f64;
        let state = if paused { "PAUSED" } else { "RUNNING" };
        window.set_title(&format!(
            "{} | FPS: '{:.1}' | UPDATE: '{:.2} ms' | RENDER: '{:.2} ms' | ALIVE: '{} ({:.1}%)' | GENERATION: '{}' | STATE: '{}'",
            window_title(),
            self.fps,
            self.average_update_ms,
            self.average_render_ms,
            alive,
            population,
            self.generation,
            state
        ));
    }
}

fn average_milliseconds(duration: Duration, samples: u64) -> f64 {
    if samples == 0 {
        0.0
    } else {
        duration.as_secs_f64() * 1_000.0 / samples as f64
    }
}
