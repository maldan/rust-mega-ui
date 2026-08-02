//! Interactive mega-ui showcase.
//!
//! ```text
//! cargo run --example demo
//! ```

use std::sync::Arc;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use mega_ui::{CursorIcon, DrawCommand, Ui, UiInput, Window};
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{Window as WinitWindow, WindowId};

const MAX_QUADS: usize = 50_000;
const MAX_VERTICES: usize = MAX_QUADS * 6;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    kind: f32,
    tex: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    viewport: [f32; 2],
    _pad: [f32; 2],
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    bind_layout: wgpu::BindGroupLayout,
    uniform_buf: wgpu::Buffer,
    vertex_buf: wgpu::Buffer,
    font_tex: wgpu::Texture,
    font_view: wgpu::TextureView,
    font_size: (u32, u32),
    sampler: wgpu::Sampler,
    tex0_view: wgpu::TextureView,
}

struct DemoState {
    name: String,
    enabled: bool,
    volume: f32,
    mode: usize,
    theme: usize,
    clicks: u32,
    plot: Vec<f32>,
    show_help: bool,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            name: String::from("mega-ui"),
            enabled: true,
            volume: 0.65,
            mode: 0,
            theme: 0,
            clicks: 0,
            plot: (0..48)
                .map(|i| ((i as f32) * 0.35).sin() * 0.5 + 0.5)
                .collect(),
            show_help: true,
        }
    }
}

struct App {
    window: Option<Arc<WinitWindow>>,
    gpu: Option<Gpu>,
    ui: Ui,
    demo: DemoState,
    input: FrameInput,
    last_frame: Instant,
    started: Instant,
    cursor: CursorIcon,
}

#[derive(Default)]
struct FrameInput {
    mouse_pos: Vec2,
    mouse_down: bool,
    mouse_pressed: bool,
    mouse_released: bool,
    scroll_delta: Vec2,
    text: String,
    key_backspace: bool,
    key_enter: bool,
    key_left: bool,
    key_right: bool,
    key_up: bool,
    key_down: bool,
    key_home: bool,
    key_end: bool,
    key_shift: bool,
    key_ctrl: bool,
    key_copy: bool,
    key_paste: bool,
    key_cut: bool,
    key_select_all: bool,
    modifiers: winit::keyboard::ModifiersState,
}

impl FrameInput {
    fn clear_edges(&mut self) {
        self.mouse_pressed = false;
        self.mouse_released = false;
        self.scroll_delta = Vec2::ZERO;
        self.text.clear();
        self.key_backspace = false;
        self.key_enter = false;
        self.key_left = false;
        self.key_right = false;
        self.key_up = false;
        self.key_down = false;
        self.key_home = false;
        self.key_end = false;
        self.key_copy = false;
        self.key_paste = false;
        self.key_cut = false;
        self.key_select_all = false;
    }

    fn to_ui(&self, viewport: Vec2, dt: f32) -> UiInput {
        UiInput {
            mouse_pos: self.mouse_pos,
            mouse_down: self.mouse_down,
            mouse_pressed: self.mouse_pressed,
            mouse_released: self.mouse_released,
            viewport,
            scroll_delta: self.scroll_delta,
            dt,
            text: self.text.clone(),
            key_backspace: self.key_backspace,
            key_enter: self.key_enter,
            key_left: self.key_left,
            key_right: self.key_right,
            key_up: self.key_up,
            key_down: self.key_down,
            key_home: self.key_home,
            key_end: self.key_end,
            key_shift: self.key_shift,
            key_ctrl: self.key_ctrl,
            key_copy: self.key_copy,
            key_paste: self.key_paste,
            key_cut: self.key_cut,
            key_select_all: self.key_select_all,
            clipboard: String::new(),
        }
    }
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            gpu: None,
            ui: Ui::new(),
            demo: DemoState::default(),
            input: FrameInput::default(),
            last_frame: Instant::now(),
            started: Instant::now(),
            cursor: CursorIcon::Default,
        }
    }

    fn init_gpu(&mut self, window: Arc<WinitWindow>) {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("mega-ui demo"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .expect("request_device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui bind layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x4,
                        3 => Float32,
                        4 => Float32,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ui uniforms"),
            contents: bytemuck::bytes_of(&Uniforms {
                viewport: [width as f32, height as f32],
                _pad: [0.0; 2],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<UiVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (pixels, atlas_w, atlas_h) = self.ui.font_atlas();
        let (font_tex, font_view) = create_font_texture(&device, &queue, pixels, atlas_w, atlas_h);
        let (_, tex0_view) = create_placeholder_rgba(&device, &queue);

        let bind_group = make_bind_group(
            &device,
            &bind_layout,
            &uniform_buf,
            &font_view,
            &sampler,
            &tex0_view,
        );

        self.gpu = Some(Gpu {
            device,
            queue,
            surface,
            config,
            pipeline,
            bind_group,
            bind_layout,
            uniform_buf,
            vertex_buf,
            font_tex,
            font_view,
            font_size: (atlas_w, atlas_h),
            sampler,
            tex0_view,
        });
        self.window = Some(window);
    }

    fn resize(&mut self, width: u32, height: u32) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        if width == 0 || height == 0 {
            return;
        }
        gpu.config.width = width;
        gpu.config.height = height;
        gpu.surface.configure(&gpu.device, &gpu.config);
        gpu.queue.write_buffer(
            &gpu.uniform_buf,
            0,
            bytemuck::bytes_of(&Uniforms {
                viewport: [width as f32, height as f32],
                _pad: [0.0; 2],
            }),
        );
    }

    fn sync_font_atlas(&mut self) {
        if !self.ui.font_atlas_take_dirty() {
            return;
        }
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let (pixels, w, h) = self.ui.font_atlas();
        if (w, h) != gpu.font_size {
            let (tex, view) = create_font_texture(&gpu.device, &gpu.queue, pixels, w, h);
            gpu.font_tex = tex;
            gpu.font_view = view;
            gpu.font_size = (w, h);
            gpu.bind_group = make_bind_group(
                &gpu.device,
                &gpu.bind_layout,
                &gpu.uniform_buf,
                &gpu.font_view,
                &gpu.sampler,
                &gpu.tex0_view,
            );
        } else {
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gpu.font_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    fn redraw(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        let viewport = Vec2::new(size.width as f32, size.height as f32);
        let (draw_list, needs_repaint, cursor) = self.build_ui(viewport, dt);
        self.apply_cursor(&window, cursor);
        self.sync_font_atlas();

        let vertices = build_vertices(&draw_list);
        let vertex_count = vertices.len() as u32;

        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };

        if !vertices.is_empty() {
            gpu.queue
                .write_buffer(&gpu.vertex_buf, 0, bytemuck::cast_slice(&vertices));
        }

        let frame = match gpu.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                window.request_redraw();
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                window.request_redraw();
                return;
            }
            Err(e) => {
                log::error!("surface error: {e}");
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ui frame"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.10,
                            g: 0.11,
                            b: 0.13,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.bind_group, &[]);
            pass.set_vertex_buffer(0, gpu.vertex_buf.slice(..));
            if vertex_count > 0 {
                pass.draw(0..vertex_count, 0..1);
            }
        }

        gpu.queue.submit(Some(encoder.finish()));
        frame.present();
        self.input.clear_edges();

        if needs_repaint {
            window.request_redraw();
        }
    }

    fn build_ui(
        &mut self,
        viewport: Vec2,
        dt: f32,
    ) -> (Vec<DrawCommand>, bool, CursorIcon) {
        let input = self.input.to_ui(viewport, dt);
        self.ui.begin_frame(input);

        let demo = &mut self.demo;
        self.ui.window(
            Window::new("Widgets")
                .pos(Vec2::new(24.0, 24.0))
                .size(Vec2::new(320.0, 420.0))
                .resizable(true)
                .collapsible(true),
            |ui| {
                ui.label("mega-ui demo");
                ui.separator();
                ui.text_input("name", &mut demo.name);
                ui.checkbox("Enabled", &mut demo.enabled);
                ui.slider("Volume", &mut demo.volume, 0.0..=1.0);
                ui.select("Mode", &mut demo.mode, &["Edit", "Play", "Inspect"]);
                ui.toggle("Theme", &mut demo.theme, &["Dark", "Light"]);
                ui.separator();
                ui.add_enabled(demo.enabled, |ui| {
                    if ui.button("Click me").clicked() {
                        demo.clicks += 1;
                    }
                });
                ui.label(&format!("Clicks: {}", demo.clicks));
                ui.separator();
                ui.label("Plot");
                ui.plot(Vec2::new(0.0, 80.0), &demo.plot);
            },
        );

        self.ui.window(
            Window::new("Help")
                .pos(Vec2::new(370.0, 24.0))
                .size(Vec2::new(280.0, 180.0))
                .open(&mut demo.show_help),
            |ui| {
                ui.label("Drag window titles to move.");
                ui.label("Resize from the bottom-right.");
                ui.label("Scroll / type in focused fields.");
                ui.separator();
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
            },
        );

        let t = self.started.elapsed().as_secs_f32();
        for (i, v) in demo.plot.iter_mut().enumerate() {
            *v = ((i as f32) * 0.35 + t).sin() * 0.45 + 0.5;
        }
        self.ui.request_repaint();

        let out = self.ui.end_frame();
        (out.draw_list, out.needs_repaint, out.cursor)
    }

    fn apply_cursor(&mut self, window: &WinitWindow, cursor: CursorIcon) {
        if cursor == self.cursor {
            return;
        }
        self.cursor = cursor;
        window.set_cursor(map_cursor(cursor));
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WinitWindow::default_attributes()
            .with_title("mega-ui demo")
            .with_inner_size(LogicalSize::new(960.0, 640.0));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("create window"),
        );
        self.init_gpu(window.clone());
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.resize(size.width, size.height);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = self.window.as_ref().map(|w| w.inner_size());
                if let Some(size) = size {
                    self.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    let down = state == ElementState::Pressed;
                    if down && !self.input.mouse_down {
                        self.input.mouse_pressed = true;
                    }
                    if !down && self.input.mouse_down {
                        self.input.mouse_released = true;
                    }
                    self.input.mouse_down = down;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x * 40.0, y * 40.0),
                    MouseScrollDelta::PixelDelta(p) => Vec2::new(p.x as f32, p.y as f32),
                };
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.input.modifiers = mods.state();
                self.input.key_shift = mods.state().shift_key();
                self.input.key_ctrl = mods.state().control_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let ctrl = self.input.modifiers.control_key();
                match &event.logical_key {
                    Key::Named(NamedKey::Backspace) => self.input.key_backspace = true,
                    Key::Named(NamedKey::Enter) => self.input.key_enter = true,
                    Key::Named(NamedKey::ArrowLeft) => self.input.key_left = true,
                    Key::Named(NamedKey::ArrowRight) => self.input.key_right = true,
                    Key::Named(NamedKey::ArrowUp) => self.input.key_up = true,
                    Key::Named(NamedKey::ArrowDown) => self.input.key_down = true,
                    Key::Named(NamedKey::Home) => self.input.key_home = true,
                    Key::Named(NamedKey::End) => self.input.key_end = true,
                    Key::Character(c) if ctrl => match c.to_lowercase().as_str() {
                        "c" => self.input.key_copy = true,
                        "v" => self.input.key_paste = true,
                        "x" => self.input.key_cut = true,
                        "a" => self.input.key_select_all = true,
                        _ => {}
                    },
                    _ => {}
                }
                if !ctrl {
                    if let Some(text) = event.text.as_ref() {
                        for ch in text.chars() {
                            if !ch.is_control() {
                                self.input.text.push(ch);
                            }
                        }
                    }
                }
                if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                    event_loop.exit();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.input.text.push_str(&text);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn build_vertices(cmds: &[DrawCommand]) -> Vec<UiVertex> {
    let mut out = Vec::with_capacity(cmds.len().min(MAX_QUADS) * 6);
    for cmd in cmds.iter().take(MAX_QUADS) {
        let x0 = cmd.rect.min.x;
        let y0 = cmd.rect.min.y;
        let x1 = cmd.rect.max.x;
        let y1 = cmd.rect.max.y;
        let u0 = cmd.uv_min[0];
        let v0 = cmd.uv_min[1];
        let u1 = cmd.uv_max[0];
        let v1 = cmd.uv_max[1];
        let color = cmd.color;
        let kind = cmd.kind;
        let tex = cmd.tex as f32;
        let quad = [
            UiVertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color,
                kind,
                tex,
            },
            UiVertex {
                pos: [x1, y0],
                uv: [u1, v0],
                color,
                kind,
                tex,
            },
            UiVertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color,
                kind,
                tex,
            },
            UiVertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color,
                kind,
                tex,
            },
            UiVertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color,
                kind,
                tex,
            },
            UiVertex {
                pos: [x0, y1],
                uv: [u0, v1],
                color,
                kind,
                tex,
            },
        ];
        out.extend_from_slice(&quad);
    }
    out
}

fn create_font_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pixels: &[u8],
    w: u32,
    h: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("font atlas"),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w.max(1)),
            rows_per_image: Some(h.max(1)),
        },
        wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_placeholder_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tex0 placeholder"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[180, 200, 255, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn make_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buf: &wgpu::Buffer,
    font_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    tex0_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ui bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(font_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(tex0_view),
            },
        ],
    })
}

fn map_cursor(icon: CursorIcon) -> winit::window::CursorIcon {
    match icon {
        CursorIcon::Default => winit::window::CursorIcon::Default,
        CursorIcon::Pointer => winit::window::CursorIcon::Pointer,
        CursorIcon::Move => winit::window::CursorIcon::Move,
        CursorIcon::ResizeNwse => winit::window::CursorIcon::NwseResize,
        CursorIcon::ResizeEw => winit::window::CursorIcon::EwResize,
        CursorIcon::ResizeNs => winit::window::CursorIcon::NsResize,
        CursorIcon::Text => winit::window::CursorIcon::Text,
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run app");
}
