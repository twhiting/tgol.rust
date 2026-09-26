use crate::life::Grid;
use log::{debug, error};
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use std::time::{Duration, Instant};
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

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::new())?;
    Ok(())
}

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    input: WinitInputHelper,
    life: Grid,
    paused: bool,
    draw_state: Option<bool>,
    debug_overlay: DebugOverlay,
}

impl App {
    fn new() -> Self {
        let mut life = Grid::new_empty(WIDTH as usize, HEIGHT as usize);
        life.randomize();

        Self {
            window: None,
            pixels: None,
            input: WinitInputHelper::new(),
            life,
            paused: false,
            draw_state: None,
            debug_overlay: DebugOverlay::new(),
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(pixels)) = (self.window.as_ref(), self.pixels.as_mut()) else {
            return;
        };

        let update_started = self.debug_overlay.enabled.then(Instant::now);
        let updating = !self.paused;
        if updating {
            let should_continue = self.life.update();
            self.debug_overlay.record_update(
                update_started
                    .map(|started| started.elapsed())
                    .unwrap_or_default(),
            );
            if !should_continue {
                self.paused = true;
            }
        }

        let render_started = self.debug_overlay.enabled.then(Instant::now);
        draw_grid(&self.life, pixels.frame_mut());

        let render_result = pixels.render();
        self.debug_overlay.record_render(
            render_started
                .map(|started| started.elapsed())
                .unwrap_or_default(),
        );
        self.debug_overlay
            .refresh_title(window, &self.life, self.paused, updating && self.paused);

        if let Err(error) = render_result {
            error!("pixels.render() failed: {}", error);
            event_loop.exit();
        }
    }

    fn handle_input(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(pixels)) = (self.window.as_ref(), self.pixels.as_mut()) else {
            return;
        };
        let input = &self.input;

        if input.key_pressed(KeyCode::Escape) || input.close_requested() || input.destroyed() {
            log::info!("Escape pressed. Quitting..");
            event_loop.exit();
            return;
        }

        if input.key_pressed(KeyCode::F3) {
            self.debug_overlay.toggle();
            if self.debug_overlay.enabled {
                self.debug_overlay
                    .refresh_title(window, &self.life, self.paused, true);
            } else {
                window.set_title(&window_title());
            }
        }

        if input.key_pressed_os(KeyCode::Space) {
            log::info!("'SPACE' pressed. Pausing..");
            self.paused = true;
            self.debug_overlay
                .refresh_title(window, &self.life, self.paused, true);
        }

        if input.key_pressed(KeyCode::KeyP) {
            log::info!("'P' pressed. Toggling pause..");
            self.paused = !self.paused;
            self.debug_overlay
                .refresh_title(window, &self.life, self.paused, true);
        }

        if input.key_pressed(KeyCode::KeyR) {
            log::info!("'R' pressed. Randomizing..");
            self.life.randomize();
            self.paused = false;
            self.debug_overlay.reset_generation();
            self.debug_overlay
                .refresh_title(window, &self.life, self.paused, true);
        }

        if input.key_pressed(KeyCode::KeyK) {
            let kill_count = self.life.randomly_kill();
            log::info!("'K' pressed. Randomly killed {:?} cells..", kill_count);
            self.paused = false;
            self.debug_overlay
                .refresh_title(window, &self.life, self.paused, true);
        }

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
            if self.paused {
                self.paused = false;
                self.debug_overlay
                    .refresh_title(window, &self.life, self.paused, true);
            }
        } else if let Some(draw_alive) = self.draw_state {
            let release = input.mouse_released(MouseButton::Left);
            let held = input.mouse_held(MouseButton::Left);

            if release || held {
                self.life.set_line(
                    mouse_prev_cell.0,
                    mouse_prev_cell.1,
                    mouse_cell.0,
                    mouse_cell.1,
                    draw_alive,
                );
            }

            if release || !held {
                debug!("Draw end");
                self.draw_state = None;
            }
        }

        let mut resized = false;
        if let Some(size) = input.window_resized() {
            log::info!(
                "Window resize. Width: {:?}, Height: {:?}",
                size.width,
                size.height
            );

            if let Err(error) = pixels.resize_surface(size.width, size.height) {
                error!("pixels.resize_surface() failed: {}", error);
                event_loop.exit();
                return;
            }
            resized = true;
        }

        if !self.paused || resized {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let size = window_size();
        let attributes = Window::default_attributes()
            .with_title(window_title())
            .with_inner_size(size)
            .with_min_inner_size(size);

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                error!("Failed to create window: {}", error);
                event_loop.exit();
                return;
            }
        };

        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());

        match Pixels::new(WIDTH, HEIGHT, surface_texture) {
            Ok(pixels) => self.pixels = Some(pixels),
            Err(error) => {
                error!("Failed to create pixel buffer: {}", error);
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
    }
}

fn draw_grid(grid: &Grid, screen: &mut [u8]) {
    debug_assert_eq!(screen.len(), 4 * grid.cell_count());

    let (pixels, remainder) = screen.as_chunks_mut::<4>();
    debug_assert!(remainder.is_empty());

    for ((alive, heat), pixel) in grid.cells().zip(pixels) {
        let color = if alive {
            [50, 0, 0xff, 0xff]
        } else {
            [
                heat.saturating_sub(100),
                0,
                heat.saturating_sub(30),
                heat.saturating_sub(30),
            ]
        };

        pixel.copy_from_slice(&color);
    }
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
