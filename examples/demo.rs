use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Theme, Window, WindowBuilder},
};

use egui_wgpu::wgpu;
use egui_wgpu::wgpu::InstanceDescriptor;

pub struct Renderer {
    gpu: Gpu,
    egui_renderer: egui_wgpu::Renderer,
}

impl Renderer {
    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let gpu = Gpu::new_async(window, width, height).await;

        let egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.surface_config.format, None, 1, false);

        Self { gpu, egui_renderer }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
    }

    pub fn render_frame(
        &mut self,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        _delta_time: crate::Duration,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let surface_texture = self
            .gpu
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: Some(self.gpu.surface_format),
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                    usage: None,
                });

        encoder.insert_debug_marker("Render scene");

        // need this block to preserve encoder ownership
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &paint_jobs,
                &screen_descriptor,
            );
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
}

impl Gpu {
    // pub fn aspect_ratio(&self) -> f32 {
    //     self.surface_config.width as f32 / self.surface_config.height.max(1) as f32
    // }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub async fn new_async(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let instance = wgpu::Instance::new(&InstanceDescriptor::default());
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");
        let (device, queue) = {
            println!("WGPU Adapter Features: {:#?}", adapter.features());
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("WGPU Device"),
                        memory_hints: wgpu::MemoryHints::default(),
                        required_features: wgpu::Features::default(),
                        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    },
                    None,
                )
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb()) // egui wants a non-srgb surface texture
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        Self {
            surface,
            device,
            queue,
            surface_config,
            surface_format,
        }
    }
}

pub struct DemoApp {
    window: Arc<Window>,
    renderer: Renderer,
    gui_state: egui_tao::State,
    last_render_time: Instant,
    last_size: (u32, u32),
    switch_position: bool,
}

impl DemoApp {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let window = WindowBuilder::new()
            .with_title("demo tao window")
            .build(event_loop)
            .expect("Failed to create window");

        let gui_context = egui::Context::default();

        let inner_size = window.inner_size();
        let last_size = (inner_size.width, inner_size.height);

        let viewport_id = gui_context.viewport_id();
        let gui_state = egui_tao::State::new(
            gui_context,
            viewport_id,
            &window,
            Some(window.scale_factor() as _),
            Some(Theme::Dark),
            None,
        );

        let (width, height) = (window.inner_size().width, window.inner_size().height);

        let window_handle = Arc::new(window);

        let renderer =
            pollster::block_on(async { Renderer::new(window_handle.clone(), width, height).await });

        Self {
            window: window_handle,
            renderer,
            gui_state,
            last_render_time: Instant::now(),
            last_size,
            switch_position: false,
        }
    }

    fn handle_event(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        match event {
            Event::WindowEvent { event, .. } => {
                // Receive gui window event
                if self
                    .gui_state
                    .on_window_event(&self.window, &event)
                    .consumed
                {
                    return;
                }

                // If the gui didn't consume the event, handle it
                match event {
                    WindowEvent::KeyboardInput { event, .. } => {
                        // Exit by pressing the escape key
                        if let tao::keyboard::KeyCode::Escape = event.physical_key {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    WindowEvent::Resized(PhysicalSize { width, height }) => {
                        println!("Resizing renderer surface to: ({width}, {height})");
                        self.renderer.resize(width, height);
                        self.last_size = (width, height);

                        let scale_factor = self.window.scale_factor() as f32;
                        self.gui_state.egui_ctx().set_pixels_per_point(scale_factor);
                    }
                    WindowEvent::CloseRequested => {
                        println!("Close requested. Exiting...");
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => (),
                }
            }
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let delta_time = now - self.last_render_time;
                self.last_render_time = now;

                let gui_input = self.gui_state.take_egui_input(&self.window);
                self.gui_state.egui_ctx().begin_pass(gui_input);

                egui::CentralPanel::default().show(self.gui_state.egui_ctx(), |ui| {
                    ui.heading("tao/egui/wgpu sample");
                    ui.checkbox(&mut self.switch_position, "an egui switch");
                });

                let egui_tao::egui::FullOutput {
                    textures_delta,
                    shapes,
                    pixels_per_point,
                    platform_output,
                    ..
                } = self.gui_state.egui_ctx().end_pass();

                self.gui_state
                    .handle_platform_output(&self.window, platform_output);

                let paint_jobs = self
                    .gui_state
                    .egui_ctx()
                    .tessellate(shapes, pixels_per_point);

                let screen_descriptor = {
                    let (width, height) = self.last_size;
                    egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [width, height],
                        pixels_per_point: self.window.scale_factor() as f32,
                    }
                };

                self.renderer.render_frame(
                    screen_descriptor,
                    paint_jobs,
                    textures_delta,
                    delta_time,
                );
                self.window.request_redraw();
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let mut app = DemoApp::new(&event_loop);

    app.window.request_redraw();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        app.handle_event(event, control_flow);
    });
}
