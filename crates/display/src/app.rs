//! The windowed / full-screen app loop.

use std::sync::Arc;
use std::time::Instant;

use chrono::{Local, Utc};
use marqueet_core::config::DisplayConfig;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::render::{self, Renderer};
use crate::scene::{FeedSource, Scene, SceneSetup};
use crate::upscale::{Upscaler, render_size};
use crate::watchdog::{self, Heartbeat};

pub fn run(config: DisplayConfig, size: (u32, u32), fullscreen: bool, source: FeedSource) -> render::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    // GLES (e.g. on Wayland / Ubuntu Frame) needs the display handle up front.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(
        event_loop.owned_display_handle(),
    )));
    let mut app = App { config, size, fullscreen, source, instance, state: None, error: None, heart: None };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct App {
    config: DisplayConfig,
    size: (u32, u32),
    fullscreen: bool,
    source: FeedSource,
    instance: wgpu::Instance,
    state: Option<State>,
    error: Option<Box<dyn std::error::Error + Send + Sync>>,
    /// Beaten every turn of the loop once the window is up (see watchdog).
    heart: Option<Heartbeat>,
}

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    scene: Scene,
    last_frame: Instant,
    stats: FrameStats,
    /// Draws below the screen's resolution and scales up (see upscale).
    upscaler: Upscaler,
    /// The size the scene draws at (the screen's, or smaller).
    render_size: (u32, u32),
    /// The resolution setting `render_size` was worked out for.
    resolution: marqueet_core::config::Resolution,
}

/// Logs frame rate and CPU time per frame every few seconds, to spot
/// hardware that can't keep up (e.g. when testing on a Raspberry Pi).
struct FrameStats {
    since: Instant,
    frames: u32,
    busy: std::time::Duration,
}

impl FrameStats {
    const INTERVAL_SECS: f64 = 10.0;

    fn record(&mut self, busy: std::time::Duration) {
        self.frames += 1;
        self.busy += busy;
        let elapsed = self.since.elapsed().as_secs_f64();
        if elapsed >= Self::INTERVAL_SECS {
            let fps = f64::from(self.frames) / elapsed;
            let cpu_ms = self.busy.as_secs_f64() * 1000.0 / f64::from(self.frames.max(1));
            log::info!("{fps:.1} fps, {cpu_ms:.2} ms CPU per frame");
            *self = FrameStats { since: Instant::now(), frames: 0, busy: Default::default() };
        }
    }
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> render::Result<State> {
        let mut attrs = Window::default_attributes()
            .with_title("Marqueet")
            .with_inner_size(PhysicalSize::new(self.size.0, self.size.1));
        if self.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = Arc::new(event_loop.create_window(attrs)?);
        if self.fullscreen {
            window.set_cursor_visible(false);
        }
        let surface = self.instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(self.instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))?;
        let (device, queue) = pollster::block_on(render::request_device(&adapter))?;

        let size = window.inner_size();
        let mut surface_config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface not supported by this GPU")?;
        let caps = surface.get_capabilities(&adapter);
        if let Some(srgb) = caps.formats.iter().copied().find(wgpu::TextureFormat::is_srgb) {
            surface_config.format = srgb;
        }
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &surface_config);

        let renderer = Renderer::new(&device, surface_config.format);
        let upscaler = Upscaler::new(&device, surface_config.format);
        let resolution = self.config.resolution;
        let render = render_size((surface_config.width, surface_config.height), resolution.max_height());
        let scene = Scene::new(
            self.config.clone(),
            render.0,
            render.1,
            SceneSetup {
                now: Utc::now(),
                tz: *Local::now().offset(),
                source: self.source.clone(),
                max_strip_width: Renderer::max_strip_width(self.config.ticker_rows),
            },
        );
        let stats = FrameStats { since: Instant::now(), frames: 0, busy: Default::default() };
        log::info!("screen {}x{}, drawing at {}x{}", surface_config.width, surface_config.height, render.0, render.1);
        Ok(State {
            window,
            surface,
            surface_config,
            device,
            queue,
            renderer,
            scene,
            last_frame: Instant::now(),
            stats,
            upscaler,
            render_size: render,
            resolution,
        })
    }
}

impl State {
    fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface.configure(&self.device, &self.surface_config);
        self.fit_render_size();
    }

    /// Works out the drawing size for the screen and the resolution
    /// setting, and lays the scene out again when it changes.
    fn fit_render_size(&mut self) {
        self.resolution = self.scene.config.resolution;
        let screen = (self.surface_config.width, self.surface_config.height);
        let size = render_size(screen, self.resolution.max_height());
        if size != self.render_size {
            log::info!("screen {}x{}, drawing at {}x{}", screen.0, screen.1, size.0, size.1);
        }
        self.render_size = size;
        self.scene.resize(size.0, size.1, Utc::now());
    }

    /// Shortest time between frames for the frame-rate setting.
    fn frame_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(1.0 / f64::from(self.scene.config.max_fps.max(1)))
    }

    fn frame(&mut self) {
        let now = Instant::now();
        // Clamp so a stall (e.g. window drag) doesn't teleport the ticker.
        let dt = now.duration_since(self.last_frame).as_secs_f64().min(0.1);
        self.last_frame = now;
        self.scene.update(dt, Utc::now());

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Validation => {
                log::warn!("surface validation error; skipping frame");
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        if self.render_size == (self.surface_config.width, self.surface_config.height) {
            self.renderer.render(&self.device, &self.queue, &mut self.scene, &view);
        } else {
            let target = self.upscaler.target(&self.device, self.render_size);
            self.renderer.render(&self.device, &self.queue, &mut self.scene, target);
            self.upscaler.draw(&self.device, &self.queue, &view);
        }
        self.stats.record(now.elapsed());
        self.window.pre_present_notify();
        self.queue.present(frame);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        match self.init(event_loop) {
            Ok(state) => {
                state.window.request_redraw();
                self.state = Some(state);
                self.heart = Some(watchdog::start());
            }
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // The loop polls, so this runs continually, even while the window is
        // hidden; a stuck frame stops it.
        if let Some(heart) = &self.heart {
            heart.beat();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key, state: ElementState::Pressed, .. }, ..
            } => match logical_key.as_ref() {
                Key::Named(NamedKey::Escape) | Key::Character("q") => event_loop.exit(),
                Key::Character("f") | Key::Named(NamedKey::F11) => {
                    let full = state.window.fullscreen().is_some();
                    state.window.set_fullscreen((!full).then_some(Fullscreen::Borderless(None)));
                    state.window.set_cursor_visible(full);
                }
                _ => {}
            },
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                // The resolution setting changed (from the server): lay out
                // again at the new size.
                if state.scene.config.resolution != state.resolution {
                    state.fit_render_size();
                }
                // Hold to the frame-rate setting (30 keeps a Pi cooler).
                let wait = state.frame_interval().saturating_sub(state.last_frame.elapsed());
                if wait > std::time::Duration::from_millis(1) {
                    std::thread::sleep(wait);
                }
                state.frame();
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}
