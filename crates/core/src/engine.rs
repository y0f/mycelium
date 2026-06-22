use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::info;
use wgpu::Color;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::bus::PluginBus;
use crate::config::MyceliumConfig;
use crate::config_watcher::ConfigWatcher;
use crate::event::{AudioFrame, BusEvent};
use crate::gpu::GpuContext;
use crate::mapping::MappingGraph;
use crate::preset::{Preset, PresetParams};
use crate::quad::{FullscreenQuad, Uniforms};

/// overlay GUI state, kept in engine to avoid crate dep cycles.
pub struct OverlayState {
    pub visible: bool,
    pub shader_names: Vec<String>,
    pub current_shader: usize,
    pub speed: f32,
    pub intensity: f32,
    pub zoom: f32,
    pub color_shift: f32,
    pub rotation_speed: f32,
    pub bass_reactivity: f32,
    pub flash_intensity: f32,
    pub brightness: f32,
    pub mapping_enabled: bool,
    pub mapping_graph: MappingGraph,
    pub preset_name: String,
    pub auto_cycle: bool,
    /// seconds between auto-cycle shader switches.
    pub auto_cycle_interval: f32,
    pub auto_cycle_timer: f32,
    pub auto_cycle_random: bool,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            visible: false,
            shader_names: Vec::new(),
            current_shader: 0,
            speed: 0.6,
            intensity: 1.0,
            zoom: 1.0,
            color_shift: 0.2,
            rotation_speed: 0.4,
            bass_reactivity: 2.0,
            flash_intensity: 0.3,
            brightness: 1.0,
            mapping_enabled: false,
            mapping_graph: MappingGraph::default(),
            preset_name: "default".to_string(),
            auto_cycle: false,
            auto_cycle_interval: 15.0,
            auto_cycle_timer: 0.0,
            auto_cycle_random: false,
        }
    }
}

pub struct ShaderSource {
    pub name: String,
    pub wgsl: String,
    /// file path for hot-reload, e.g. "assets/shaders/fractal.wgsl".
    pub path: Option<String>,
}

pub trait ScriptEvaluator: Send {
    fn update(&self, audio: &AudioFrame, time: f32, uniforms: &mut Uniforms);
}

pub struct EngineParams {
    pub config: MyceliumConfig,
    pub shaders: Vec<ShaderSource>,
    pub bus: PluginBus,
    pub script: Option<Box<dyn ScriptEvaluator>>,
    /// handles to keep alive for the engine's lifetime, e.g. audio thread.
    pub _keep_alive: Vec<Box<dyn std::any::Any + Send>>,
    pub start_shader: Option<String>,
}

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    config: MyceliumConfig,
    shader_sources: Vec<ShaderSource>,
    quads: Vec<FullscreenQuad>,
    start_time: std::time::Instant,
    last_frame_time: std::time::Instant,
    fps: f32,
    frame_count: u64,
    bus: PluginBus,
    audio_frame: AudioFrame,
    event_buf: Vec<BusEvent>,
    config_shared: Option<Arc<RwLock<MyceliumConfig>>>,
    _config_watcher: Option<ConfigWatcher>,
    _keep_alive: Vec<Box<dyn std::any::Any + Send>>,
    script: Option<Box<dyn ScriptEvaluator>>,
    start_shader: Option<String>,
    shader_watcher: Option<notify::RecommendedWatcher>,
    shader_reload_rx: Option<crossbeam::channel::Receiver<()>>,
    egui_ctx: egui::Context,
    egui_winit: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    overlay: OverlayState,
}

impl App {
    fn new(params: EngineParams) -> Self {
        Self {
            window: None,
            gpu: None,
            config: params.config,
            shader_sources: params.shaders,
            quads: Vec::new(),
            start_time: std::time::Instant::now(),
            last_frame_time: std::time::Instant::now(),
            fps: 0.0,
            frame_count: 0,
            bus: params.bus,
            audio_frame: AudioFrame::default(),
            event_buf: Vec::with_capacity(4096),
            config_shared: None,
            _config_watcher: None,
            _keep_alive: params._keep_alive,
            script: params.script,
            start_shader: params.start_shader,
            shader_watcher: None,
            shader_reload_rx: None,
            egui_ctx: egui::Context::default(),
            egui_winit: None,
            egui_renderer: None,
            overlay: OverlayState::default(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("Mycelium")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.config.window.width,
                self.config.window.height,
            ));

        // expect() is intentional: ApplicationHandler::resumed() cannot return errors
        // (winit API constraint), and the app cannot function without a window and GPU
        // context, so panicking is correct.
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );
        let gpu = pollster::block_on(GpuContext::new(Arc::clone(&window)))
            .expect("Failed to initialize GPU");

        let quads: Vec<FullscreenQuad> = self
            .shader_sources
            .iter()
            .map(|s| FullscreenQuad::new(&gpu.device, gpu.format(), &s.wgsl))
            .collect();

        self.overlay.shader_names = self
            .shader_sources
            .iter()
            .map(|s| s.name.clone())
            .collect();

        let names: Vec<&str> = self.shader_sources.iter().map(|s| s.name.as_str()).collect();
        info!("Loaded {} shaders: {:?}", quads.len(), names);

        self.quads = quads;
        if let Some(ref name) = self.start_shader {
            if let Some(idx) = self.overlay.shader_names.iter().position(|n| n == name) {
                self.overlay.current_shader = idx;
            } else {
                tracing::warn!("Shader '{}' not found, using first shader", name);
                self.overlay.current_shader = 0;
            }
        } else {
            self.overlay.current_shader = 0;
        }
        apply_shader_defaults(&mut self.overlay);
        self.start_time = std::time::Instant::now();

        let egui_winit = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.format(),
            egui_wgpu::RendererOptions::default(),
        );
        self.egui_winit = Some(egui_winit);
        self.egui_renderer = Some(egui_renderer);

        let config_arc = Arc::new(RwLock::new(self.config.clone()));
        let watcher = ConfigWatcher::new(
            PathBuf::from("config/default.toml"),
            self.bus.event_sender(),
            Arc::clone(&config_arc),
        )
        .ok();
        self.config_shared = Some(config_arc);
        self._config_watcher = watcher;

        {
            let (tx, rx) = crossbeam::channel::bounded(4);
            let watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
                if let Ok(event) = res {
                    if matches!(event.kind, notify::EventKind::Modify(_)) {
                        let _ = tx.try_send(());
                    }
                }
            });
            if let Ok(mut w) = watcher {
                let shader_dir = PathBuf::from("assets/shaders");
                if shader_dir.exists() {
                    let _ = notify::Watcher::watch(&mut w, &shader_dir, notify::RecursiveMode::NonRecursive);
                    self.shader_watcher = Some(w);
                    self.shader_reload_rx = Some(rx);
                    info!("Shader hot-reload watching assets/shaders/");
                }
            }
        }

        info!("Window and GPU initialized (Tab to toggle GUI)");
        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // intercept Tab and Escape before egui gets them.
        if let WindowEvent::KeyboardInput { ref event, .. } = &event {
            if event.state == winit::event::ElementState::Pressed {
                use winit::keyboard::{Key, NamedKey};
                match &event.logical_key {
                    Key::Named(NamedKey::Tab) => {
                        self.overlay.visible = !self.overlay.visible;
                        return;
                    }
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    Key::Named(NamedKey::F11) => {
                        if let Some(window) = &self.window {
                            let is_fullscreen = window.fullscreen().is_some();
                            if is_fullscreen {
                                window.set_fullscreen(None);
                            } else {
                                window.set_fullscreen(Some(
                                    winit::window::Fullscreen::Borderless(None),
                                ));
                            }
                        }
                        return;
                    }
                    Key::Character(c) if !self.overlay.visible => {
                        if let Some(digit) = c.as_str().chars().next().and_then(|ch| ch.to_digit(10)) {
                            // keys 1-9 map to shaders 0-8, key 0 maps to shader 9.
                            let idx = if digit == 0 { 9 } else { (digit - 1) as usize };
                            if idx < self.quads.len() {
                                self.overlay.current_shader = idx;
                                apply_shader_defaults(&mut self.overlay);
                                return;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if let (Some(egui_winit), Some(window)) = (&mut self.egui_winit, &self.window) {
            let response = egui_winit.on_window_event(window, &event);
            if response.consumed && self.overlay.visible && !matches!(event, WindowEvent::RedrawRequested) {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(gpu) = &self.gpu else { return };
                let Some(window) = &self.window else { return };

                let now = std::time::Instant::now();
                let dt = now.duration_since(self.last_frame_time).as_secs_f32();
                self.last_frame_time = now;
                if dt > 0.0 {
                    self.fps = self.fps * 0.95 + (1.0 / dt) * 0.05; // EMA smoothed
                }
                self.frame_count += 1;

                if self.overlay.auto_cycle && !self.quads.is_empty() {
                    self.overlay.auto_cycle_timer += dt;
                    if self.overlay.auto_cycle_timer >= self.overlay.auto_cycle_interval {
                        self.overlay.auto_cycle_timer = 0.0;
                        if self.overlay.auto_cycle_random {
                            let next = (self.frame_count as usize * 7 + 3) % self.quads.len();
                            self.overlay.current_shader = next;
                        } else {
                            self.overlay.current_shader =
                                (self.overlay.current_shader + 1) % self.quads.len();
                        }
                        apply_shader_defaults(&mut self.overlay);
                    }
                }

                if let Some(rx) = &self.shader_reload_rx {
                    if rx.try_recv().is_ok() {
                        // drain any extra notifications.
                        while rx.try_recv().is_ok() {}
                        info!("Shader change detected, reloading...");
                        let mut reloaded = Vec::new();
                        let mut all_ok = true;
                        for source in &self.shader_sources {
                            if let Some(path) = &source.path {
                                match std::fs::read_to_string(path) {
                                    Ok(wgsl) => {
                                        // validate WGSL before sending to GPU to prevent panics.
                                        if naga::front::wgsl::parse_str(&wgsl).is_err() {
                                            tracing::warn!(
                                                "Invalid WGSL in {}, keeping old shader",
                                                source.name
                                            );
                                            all_ok = false;
                                            break;
                                        }
                                        reloaded.push(FullscreenQuad::new(
                                            &gpu.device,
                                            gpu.format(),
                                            &wgsl,
                                        ));
                                        info!("Reloaded: {}", source.name);
                                    }
                                    Err(_) => {
                                        tracing::warn!(
                                            "Failed to reload {}, keeping old",
                                            source.name
                                        );
                                        all_ok = false;
                                        break;
                                    }
                                }
                            } else {
                                // no file path, rebuild from embedded source.
                                reloaded.push(FullscreenQuad::new(&gpu.device, gpu.format(), &source.wgsl));
                            }
                        }
                        if all_ok && reloaded.len() == self.shader_sources.len() {
                            self.quads = reloaded;
                        }
                    }
                }

                self.bus.read_audio(&mut self.audio_frame);

                self.bus.drain_events(&mut self.event_buf);
                for ev in &self.event_buf {
                    match ev {
                        BusEvent::ConfigReload => {
                            if let Some(config_shared) = &self.config_shared {
                                let config = config_shared.read();
                                info!(
                                    "Config reloaded: {}x{} @ {}fps",
                                    config.window.width,
                                    config.window.height,
                                    config.window.target_fps
                                );
                            }
                        }
                        // MIDI/OSC/gamepad CC 0-7 map to the 8 shader params in order.
                        BusEvent::Control(crate::event::ControlEvent::Continuous {
                            channel,
                            value,
                            ..
                        }) => {
                            match channel {
                                0 => self.overlay.speed = value * 5.0,
                                1 => self.overlay.intensity = value * 3.0,
                                2 => self.overlay.zoom = 0.1 + value * 9.9,
                                3 => self.overlay.color_shift = *value,
                                4 => self.overlay.rotation_speed = value * 5.0,
                                5 => self.overlay.bass_reactivity = value * 5.0,
                                6 => self.overlay.flash_intensity = *value,
                                7 => self.overlay.brightness = value * 3.0,
                                // CC 10-16: switch shader.
                                10..=16 => {
                                    let idx = (*channel - 10) as usize;
                                    if idx < self.quads.len() {
                                        self.overlay.current_shader = idx;
                                    }
                                }
                                _ => {}
                            }
                        }
                        // trigger events (note on, button press) advance to next shader.
                        BusEvent::Control(crate::event::ControlEvent::Trigger { .. }) => {
                            self.overlay.current_shader =
                                (self.overlay.current_shader + 1) % self.quads.len().max(1);
                            apply_shader_defaults(&mut self.overlay);
                        }
                        _ => {}
                    }
                }

                let output = match gpu.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        gpu.surface.configure(&gpu.device, &gpu.surface_config);
                        return;
                    }
                    Err(e) => {
                        tracing::error!("Surface error: {e}");
                        return;
                    }
                };

                let view = output.texture.create_view(&Default::default());
                let mut encoder = gpu.device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("render"),
                    },
                );

                let af = &self.audio_frame;
                let mut uniforms = Uniforms {
                    time: self.start_time.elapsed().as_secs_f32(),
                    res_x: gpu.surface_config.width as f32,
                    res_y: gpu.surface_config.height as f32,
                    beat_phase: af.beat_phase,
                    band0: af.bands[0], band1: af.bands[1], band2: af.bands[2],
                    band3: af.bands[3], band4: af.bands[4], band5: af.bands[5],
                    bpm: af.bpm,
                    onset: if af.onset { 1.0 } else { 0.0 },
                    sband0: af.smoothed_bands[0], sband1: af.smoothed_bands[1],
                    sband2: af.smoothed_bands[2], sband3: af.smoothed_bands[3],
                    sband4: af.smoothed_bands[4], sband5: af.smoothed_bands[5],
                    energy: af.energy,
                    spectral_centroid: af.spectral_centroid,
                    pband0: af.peak_bands[0], pband1: af.peak_bands[1],
                    pband2: af.peak_bands[2], pband3: af.peak_bands[3],
                    ..Default::default()
                };

                uniforms.speed = self.overlay.speed;
                uniforms.intensity = self.overlay.intensity;
                uniforms.zoom = self.overlay.zoom;
                uniforms.color_shift = self.overlay.color_shift;
                uniforms.rotation_speed = self.overlay.rotation_speed;
                uniforms.bass_reactivity = self.overlay.bass_reactivity;
                uniforms.flash_intensity = self.overlay.flash_intensity;
                uniforms.brightness = self.overlay.brightness;

                // audio->param mappings override GUI values for mapped params.
                if self.overlay.mapping_enabled {
                    let time = self.start_time.elapsed().as_secs_f32();
                    let result = self.overlay.mapping_graph.evaluate(&self.audio_frame, time);
                    result.apply_to(&mut uniforms);
                }

                // Lua script overrides have highest priority.
                if let Some(script) = &self.script {
                    let time = self.start_time.elapsed().as_secs_f32();
                    script.update(&self.audio_frame, time, &mut uniforms);
                }

                if let Some(quad) = self.quads.get(self.overlay.current_shader) {
                    quad.update_uniforms(&gpu.queue, &uniforms);

                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("quad"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(Color {
                                        r: 0.0,
                                        g: 0.0,
                                        b: 0.0,
                                        a: 1.0,
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: None,
                            ..Default::default()
                        });
                        pass.set_pipeline(quad.pipeline());
                        pass.set_bind_group(0, quad.bind_group(), &[]);
                        pass.draw(0..3, 0..1);
                    }
                }

                gpu.queue.submit(std::iter::once(encoder.finish()));

                // separate encoder to avoid lifetime issues.
                if let (Some(egui_winit), Some(egui_renderer)) =
                    (&mut self.egui_winit, &mut self.egui_renderer)
                {
                    let raw_input = egui_winit.take_egui_input(window);
                    let fps = self.fps;
                    let full_output = self.egui_ctx.run(raw_input, |ctx| {
                        // FPS counter, top-right, always shown.
                        egui::Area::new(egui::Id::new("fps"))
                            .fixed_pos(egui::pos2(ctx.input(|i| i.viewport_rect()).max.x - 120.0, 5.0))
                            .interactable(false)
                            .order(egui::Order::Foreground)
                            .show(ctx, |ui| {
                                ui.label(egui::RichText::new(format!("FPS: {:.0}", fps))
                                    .color(egui::Color32::from_rgba_unmultiplied(200, 200, 200, 180))
                                    .size(14.0));
                            });

                        if self.overlay.visible {
                            draw_overlay(ctx, &mut self.overlay, &self.audio_frame);
                        }
                    });

                    egui_winit.handle_platform_output(window, full_output.platform_output);

                    let tris = self
                        .egui_ctx
                        .tessellate(full_output.shapes, full_output.pixels_per_point);

                    for (id, image_delta) in &full_output.textures_delta.set {
                        egui_renderer.update_texture(&gpu.device, &gpu.queue, *id, image_delta);
                    }

                    let screen = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [
                            gpu.surface_config.width,
                            gpu.surface_config.height,
                        ],
                        pixels_per_point: full_output.pixels_per_point,
                    };

                    let mut egui_encoder = gpu.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("egui"),
                        },
                    );

                    egui_renderer.update_buffers(
                        &gpu.device,
                        &gpu.queue,
                        &mut egui_encoder,
                        &tris,
                        &screen,
                    );

                    let mut pass =
                        egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("egui_pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: None,
                            ..Default::default()
                        })
                        .forget_lifetime();
                    egui_renderer.render(&mut pass, &tris, &screen);
                    drop(pass);

                    gpu.queue.submit(std::iter::once(egui_encoder.finish()));

                    for id in &full_output.textures_delta.free {
                        egui_renderer.free_texture(id);
                    }
                }

                output.present();

                window.request_redraw();
            }
            _ => {}
        }
    }
}

fn draw_overlay(ctx: &egui::Context, state: &mut OverlayState, audio: &AudioFrame) {
    egui::SidePanel::left("mycelium_panel")
        .default_width(320.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Mycelium");
            ui.label("Tab to hide | F11 fullscreen");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Shader").strong());
                if ui.button("<").clicked() && state.current_shader > 0 {
                    state.current_shader -= 1;
                }
                let name = state
                    .shader_names
                    .get(state.current_shader)
                    .map(|s| s.as_str())
                    .unwrap_or("none");
                ui.label(egui::RichText::new(name).strong().size(16.0));
                if ui.button(">").clicked()
                    && state.current_shader + 1 < state.shader_names.len()
                {
                    state.current_shader += 1;
                }
            });
            ui.separator();

            // spectrum visualizer: 64 bars from the FFT.
            ui.label(egui::RichText::new("Spectrum").strong());
            let spec_height = 50.0;
            let spec_width = ui.available_width();
            let (spec_rect, _) = ui.allocate_exact_size(
                egui::vec2(spec_width, spec_height),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(spec_rect);
            painter.rect_filled(spec_rect, 2.0, egui::Color32::from_gray(20));

            let num_bars = 64.min(audio.spectrum_len);
            if num_bars > 0 {
                let bar_w = spec_width / num_bars as f32;
                let bin_step = audio.spectrum_len / num_bars;
                for i in 0..num_bars {
                    let bin = i * bin_step;
                    let mag = (audio.spectrum[bin] * 5.0).min(1.0);
                    let bar_h = mag * spec_height;
                    let x = spec_rect.min.x + i as f32 * bar_w;
                    let y = spec_rect.max.y - bar_h;

                    // purple->cyan gradient across frequency.
                    let t = i as f32 / num_bars as f32;
                    let r = (120.0 + 100.0 * (1.0 - t)) as u8;
                    let g = (40.0 + 180.0 * t) as u8;
                    let b = (200.0 + 55.0 * (1.0 - t * 0.5)) as u8;

                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(x, y),
                        egui::vec2(bar_w - 1.0, bar_h),
                    );
                    painter.rect_filled(bar_rect, 0.0, egui::Color32::from_rgb(r, g, b));
                }
            }
            ui.separator();

            ui.label(egui::RichText::new("Bands").strong());
            let band_names = ["Sub", "Bass", "Low", "Mid", "High", "Air"];
            for (i, name) in band_names.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{:>4}", name));
                    let smoothed = audio.smoothed_bands[i].min(2.0) / 2.0;
                    let bar = egui::ProgressBar::new(smoothed)
                        .desired_width(ui.available_width() - 10.0);
                    ui.add(bar);
                });
            }
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(format!("BPM: {:.0}", audio.bpm));
                ui.label(format!("Energy: {:.2}", audio.energy));
                if audio.onset {
                    ui.label(
                        egui::RichText::new("ONSET")
                            .color(egui::Color32::from_rgb(255, 100, 200)),
                    );
                }
            });
            ui.label(format!("Centroid: {:.0} Hz", audio.spectral_centroid));
            ui.separator();

            ui.label(egui::RichText::new("Parameters").strong());
            ui.add(egui::Slider::new(&mut state.speed, 0.0..=5.0).text("Speed"));
            ui.add(egui::Slider::new(&mut state.intensity, 0.0..=3.0).text("Intensity"));
            ui.add(
                egui::Slider::new(&mut state.zoom, 0.1..=10.0)
                    .text("Zoom")
                    .logarithmic(true),
            );
            ui.add(egui::Slider::new(&mut state.color_shift, 0.0..=1.0).text("Color Shift"));
            ui.add(egui::Slider::new(&mut state.rotation_speed, 0.0..=5.0).text("Rotation"));
            ui.add(egui::Slider::new(&mut state.bass_reactivity, 0.0..=5.0).text("Bass React"));
            ui.add(egui::Slider::new(&mut state.flash_intensity, 0.0..=1.0).text("Flash"));
            ui.add(egui::Slider::new(&mut state.brightness, 0.0..=3.0).text("Brightness"));

            if ui.button("Reset Defaults").clicked() {
                apply_shader_defaults(state);
            }
            ui.separator();

            ui.checkbox(&mut state.mapping_enabled, "Audio Mapping");
            if state.mapping_enabled {
                ui.label(
                    egui::RichText::new(format!("{} active mappings", state.mapping_graph.mappings.len()))
                        .small()
                );
            }
            ui.separator();

            ui.label(egui::RichText::new("Auto-Cycle").strong());
            ui.checkbox(&mut state.auto_cycle, "Enabled");
            if state.auto_cycle {
                ui.add(egui::Slider::new(&mut state.auto_cycle_interval, 2.0..=60.0)
                    .text("Interval (s)"));
                ui.checkbox(&mut state.auto_cycle_random, "Random order");
                let remaining = state.auto_cycle_interval - state.auto_cycle_timer;
                ui.label(egui::RichText::new(format!("Next in: {:.0}s", remaining)).small());
            }
            ui.separator();

            ui.collapsing("Keyboard Shortcuts", |ui| {
                ui.label("Tab       - Toggle GUI");
                ui.label("F11       - Fullscreen");
                ui.label("1-0       - Switch shader");
                ui.label("Escape    - Quit");
                ui.label("Ctrl+S    - Save preset");
            });
            ui.separator();

            ui.label(egui::RichText::new("Presets").strong());
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.preset_name);
            });
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let shader_name = state.shader_names
                        .get(state.current_shader)
                        .cloned()
                        .unwrap_or_default();
                    let preset = Preset {
                        name: state.preset_name.clone(),
                        shader: shader_name,
                        params: PresetParams {
                            speed: state.speed,
                            intensity: state.intensity,
                            zoom: state.zoom,
                            color_shift: state.color_shift,
                            rotation_speed: state.rotation_speed,
                            bass_reactivity: state.bass_reactivity,
                            flash_intensity: state.flash_intensity,
                            brightness: state.brightness,
                        },
                        mappings: state.mapping_graph.clone(),
                    };
                    let path = PathBuf::from(format!("presets/{}.toml", state.preset_name));
                    if let Err(e) = preset.save(&path) {
                        tracing::error!("Failed to save preset: {e}");
                    }
                }
                if ui.button("Load").clicked() {
                    let path = PathBuf::from(format!("presets/{}.toml", state.preset_name));
                    match Preset::load(&path) {
                        Ok(preset) => {
                            state.speed = preset.params.speed;
                            state.intensity = preset.params.intensity;
                            state.zoom = preset.params.zoom;
                            state.color_shift = preset.params.color_shift;
                            state.rotation_speed = preset.params.rotation_speed;
                            state.bass_reactivity = preset.params.bass_reactivity;
                            state.flash_intensity = preset.params.flash_intensity;
                            state.brightness = preset.params.brightness;
                            state.mapping_graph = preset.mappings;
                            if let Some(idx) = state.shader_names.iter().position(|n| *n == preset.shader) {
                                state.current_shader = idx;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to load preset: {e}");
                        }
                    }
                }
            });
            });
        });
}

/// apply per-shader recommended defaults when switching effects.
fn apply_shader_defaults(state: &mut OverlayState) {
    let name = state
        .shader_names
        .get(state.current_shader)
        .map(|s| s.as_str())
        .unwrap_or("");

    // tuned defaults: speed, intensity, zoom, color_shift, rotation, bass_react, flash, brightness.
    let (spd, int, zm, cs, rot, br, fl, brt) = match name {
        "fractal"      => (3.7, 2.2, 1.0, 0.46, 3.8, 1.4, 0.7, 2.1),
        "hypnotic"     => (1.0, 1.2, 1.0, 0.4, 0.6, 2.5, 0.3, 1.0),
        "voronoi"      => (0.5, 1.0, 1.5, 0.3, 0.3, 2.0, 0.3, 1.0),
        "kaleidoscope" => (0.6, 1.2, 0.8, 0.3, 0.5, 2.0, 0.3, 1.0),
        "neural"       => (0.4, 1.0, 1.2, 0.5, 0.2, 1.5, 0.3, 1.0),
        "strobe"       => (1.5, 1.5, 1.0, 0.0, 1.0, 3.0, 0.3, 1.0),
        "geometry"     => (0.5, 1.0, 1.5, 0.3, 0.4, 2.0, 0.3, 1.0),
        "nebula"       => (0.3, 1.0, 1.0, 0.3, 0.1, 1.5, 0.3, 1.0),
        "electric"     => (0.8, 1.2, 1.0, 0.2, 0.3, 2.5, 0.3, 1.0),
        "liquid"       => (0.5, 1.0, 1.0, 0.4, 0.2, 2.0, 0.3, 1.0),
        _              => (0.6, 1.0, 1.0, 0.2, 0.4, 2.0, 0.3, 1.0),
    };

    state.speed = spd;
    state.intensity = int;
    state.zoom = zm;
    state.color_shift = cs;
    state.rotation_speed = rot;
    state.bass_reactivity = br;
    state.flash_intensity = fl;
    state.brightness = brt;
}

pub fn run(params: EngineParams) -> Result<(), crate::error::MyceliumError> {
    let event_loop =
        EventLoop::new().map_err(|e| crate::error::MyceliumError::GpuInit(e.to_string()))?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::new(params);
    event_loop
        .run_app(&mut app)
        .map_err(|e| crate::error::MyceliumError::GpuInit(e.to_string()))?;

    Ok(())
}
