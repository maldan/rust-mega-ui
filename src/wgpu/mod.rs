//! Optional wgpu helpers for drawing mega-ui command lists.
//!
//! Enable with `mega-ui = { features = ["wgpu"] }`, then:
//!
//! ```ignore
//! let mut renderer = mega_ui::wgpu::UiRenderer::new(&device, &queue, format, &ui);
//! renderer.sync_atlases(&device, &queue, &mut ui);
//! renderer.set_viewport(&queue, width, height);
//! // in a render pass:
//! let stats = renderer.draw(&queue, &mut pass, &out.draw_list);
//! ```
//!
//! Host textures (`kind = 1`) use a single `tex0` binding. Consecutive commands
//! with different [`DrawCommand::tex`] slots become separate draw batches with a
//! rebind in between.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use ::wgpu::util::DeviceExt;

use crate::types::DrawCommand;
use crate::widgets::color_picker::TEX_SLOT_COLOR_SV;
use crate::Ui;

/// Soft cap on quads uploaded per frame (matches historical demo host).
pub const MAX_QUADS: usize = 50_000;
const MAX_VERTICES: usize = MAX_QUADS * 6;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    kind: f32,
    tex: f32,
    params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    viewport: [f32; 2],
    _pad: [f32; 2],
}

struct TexSlot {
    /// Owned GPU texture when uploaded via [`UiRenderer::set_texture_rgba`].
    _texture: Option<::wgpu::Texture>,
    view: ::wgpu::TextureView,
    /// Pixel size for owned RGBA uploads (`None` for external views).
    size: Option<(u32, u32)>,
    bind_group: ::wgpu::BindGroup,
}

/// Stats from the last [`UiRenderer::draw`] / [`UiRenderer::prepare`].
#[derive(Clone, Copy, Debug, Default)]
pub struct DrawStats {
    /// Draw commands submitted by UI (may exceed GPU cap).
    pub commands: usize,
    /// GPU draw calls after batching by texture slot.
    pub batches: usize,
    /// Quads actually uploaded (`min(commands, MAX_QUADS)`).
    pub quads: usize,
}

/// Pipeline + atlases + batched draw for mega-ui.
pub struct UiRenderer {
    pipeline: ::wgpu::RenderPipeline,
    bind_layout: ::wgpu::BindGroupLayout,
    uniform_buf: ::wgpu::Buffer,
    vertex_buf: ::wgpu::Buffer,
    font_tex: ::wgpu::Texture,
    font_view: ::wgpu::TextureView,
    font_size: (u32, u32),
    sampler: ::wgpu::Sampler,
    /// Fallback when a batch has no host texture (font/solid/SDF only).
    default_bind_group: ::wgpu::BindGroup,
    _placeholder_tex: ::wgpu::Texture,
    placeholder_view: ::wgpu::TextureView,
    tex_slots: HashMap<u32, TexSlot>,
    /// Quads prepared by the last [`Self::prepare`] / [`Self::draw`].
    prepared_quads: usize,
}

impl UiRenderer {
    /// Create pipeline, buffers, and initial font + color-SV atlases from `ui`.
    pub fn new(
        device: &::wgpu::Device,
        queue: &::wgpu::Queue,
        target_format: ::wgpu::TextureFormat,
        ui: &Ui,
    ) -> Self {
        let shader = device.create_shader_module(::wgpu::ShaderModuleDescriptor {
            label: Some("mega-ui"),
            source: ::wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });

        let bind_layout = device.create_bind_group_layout(&::wgpu::BindGroupLayoutDescriptor {
            label: Some("mega-ui bind layout"),
            entries: &[
                ::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ::wgpu::ShaderStages::VERTEX,
                    ty: ::wgpu::BindingType::Buffer {
                        ty: ::wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                ::wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ::wgpu::ShaderStages::FRAGMENT,
                    ty: ::wgpu::BindingType::Texture {
                        sample_type: ::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: ::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                ::wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ::wgpu::ShaderStages::FRAGMENT,
                    ty: ::wgpu::BindingType::Sampler(::wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                ::wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ::wgpu::ShaderStages::FRAGMENT,
                    ty: ::wgpu::BindingType::Texture {
                        sample_type: ::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: ::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
            label: Some("mega-ui pipeline layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&::wgpu::RenderPipelineDescriptor {
            label: Some("mega-ui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: ::wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(::wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as u64,
                    step_mode: ::wgpu::VertexStepMode::Vertex,
                    attributes: &::wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x4,
                        3 => Float32,
                        4 => Float32,
                        5 => Float32x4,
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(::wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(::wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(::wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: ::wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: ::wgpu::PrimitiveState {
                topology: ::wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: ::wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let uniform_buf = device.create_buffer_init(&::wgpu::util::BufferInitDescriptor {
            label: Some("mega-ui uniforms"),
            contents: bytemuck::bytes_of(&Uniforms {
                viewport: [1.0, 1.0],
                _pad: [0.0; 2],
            }),
            usage: ::wgpu::BufferUsages::UNIFORM | ::wgpu::BufferUsages::COPY_DST,
        });

        let vertex_buf = device.create_buffer(&::wgpu::BufferDescriptor {
            label: Some("mega-ui vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<UiVertex>()) as u64,
            usage: ::wgpu::BufferUsages::VERTEX | ::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&::wgpu::SamplerDescriptor {
            label: Some("mega-ui sampler"),
            address_mode_u: ::wgpu::AddressMode::ClampToEdge,
            address_mode_v: ::wgpu::AddressMode::ClampToEdge,
            address_mode_w: ::wgpu::AddressMode::ClampToEdge,
            mag_filter: ::wgpu::FilterMode::Linear,
            min_filter: ::wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (pixels, atlas_w, atlas_h) = ui.font_atlas();
        let (font_tex, font_view) = create_font_texture(device, queue, pixels, atlas_w, atlas_h);
        let (placeholder_tex, placeholder_view) = create_placeholder_rgba(device, queue);

        let default_bind_group = make_bind_group(
            device,
            &bind_layout,
            &uniform_buf,
            &font_view,
            &sampler,
            &placeholder_view,
        );

        let mut renderer = Self {
            pipeline,
            bind_layout,
            uniform_buf,
            vertex_buf,
            font_tex,
            font_view,
            font_size: (atlas_w, atlas_h),
            sampler,
            default_bind_group,
            _placeholder_tex: placeholder_tex,
            placeholder_view,
            tex_slots: HashMap::new(),
            prepared_quads: 0,
        };

        let (sv_pixels, sv_w, sv_h) = ui.color_sv_atlas();
        renderer.set_texture_rgba(
            device,
            queue,
            TEX_SLOT_COLOR_SV,
            sv_pixels,
            sv_w,
            sv_h,
            "color sv",
        );

        renderer
    }

    /// Update the viewport uniform (screen size in pixels).
    pub fn set_viewport(&self, queue: &::wgpu::Queue, width: f32, height: f32) {
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::bytes_of(&Uniforms {
                viewport: [width.max(1.0), height.max(1.0)],
                _pad: [0.0; 2],
            }),
        );
    }

    /// Sync font atlas and color-picker SV atlas from `ui` when dirty.
    pub fn sync_atlases(&mut self, device: &::wgpu::Device, queue: &::wgpu::Queue, ui: &mut Ui) {
        self.sync_font_atlas(device, queue, ui);
        self.sync_color_sv_atlas(device, queue, ui);
    }

    /// Upload or replace an owned RGBA8 texture for `slot` (`DrawCommand.tex`).
    pub fn set_texture_rgba(
        &mut self,
        device: &::wgpu::Device,
        queue: &::wgpu::Queue,
        slot: u32,
        pixels: &[u8],
        w: u32,
        h: u32,
        label: &str,
    ) {
        let w = w.max(1);
        let h = h.max(1);
        if let Some(existing) = self.tex_slots.get_mut(&slot) {
            if existing.size == Some((w, h)) {
                if let Some(texture) = existing._texture.as_ref() {
                    queue.write_texture(
                        ::wgpu::TexelCopyTextureInfo {
                            texture,
                            mip_level: 0,
                            origin: ::wgpu::Origin3d::ZERO,
                            aspect: ::wgpu::TextureAspect::All,
                        },
                        pixels,
                        ::wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(w * 4),
                            rows_per_image: Some(h),
                        },
                        ::wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    return;
                }
            }
        }

        let (texture, view) = create_rgba_texture(device, queue, pixels, w, h, label);
        let bind_group = make_bind_group(
            device,
            &self.bind_layout,
            &self.uniform_buf,
            &self.font_view,
            &self.sampler,
            &view,
        );
        self.tex_slots.insert(
            slot,
            TexSlot {
                _texture: Some(texture),
                view,
                size: Some((w, h)),
                bind_group,
            },
        );
    }

    /// Bind an external texture view to `slot` (e.g. offscreen scene).
    ///
    /// The view must outlive draws that reference this slot (store it yourself
    /// or recreate the bind each frame).
    pub fn bind_texture_view(
        &mut self,
        device: &::wgpu::Device,
        slot: u32,
        view: ::wgpu::TextureView,
    ) {
        let bind_group = make_bind_group(
            device,
            &self.bind_layout,
            &self.uniform_buf,
            &self.font_view,
            &self.sampler,
            &view,
        );
        self.tex_slots.insert(
            slot,
            TexSlot {
                _texture: None,
                view,
                size: None,
                bind_group,
            },
        );
    }

    /// Upload vertices for `cmds` (capped at [`MAX_QUADS`]). Call before [`Self::render`].
    pub fn prepare(&mut self, queue: &::wgpu::Queue, cmds: &[DrawCommand]) -> DrawStats {
        let stats = count_draw_stats(cmds);
        let vertices = build_vertices(cmds);
        self.prepared_quads = stats.quads;
        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(&vertices));
        }
        stats
    }

    /// Issue batched draws for the last [`Self::prepare`] into an active pass.
    ///
    /// Sets pipeline + vertex buffer. Does not begin/end the pass.
    pub fn render<'a>(&'a self, pass: &mut ::wgpu::RenderPass<'a>, cmds: &[DrawCommand]) {
        let quads = self.prepared_quads.min(cmds.len()).min(MAX_QUADS);
        if quads == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        draw_batched(pass, self, &cmds[..quads]);
    }

    /// [`Self::prepare`] + [`Self::render`] in one call.
    pub fn draw<'a>(
        &'a mut self,
        queue: &::wgpu::Queue,
        pass: &mut ::wgpu::RenderPass<'a>,
        cmds: &'a [DrawCommand],
    ) -> DrawStats {
        let stats = self.prepare(queue, cmds);
        self.render(pass, cmds);
        stats
    }

    fn sync_font_atlas(&mut self, device: &::wgpu::Device, queue: &::wgpu::Queue, ui: &mut Ui) {
        if !ui.font_atlas_take_dirty() {
            return;
        }
        let (pixels, w, h) = ui.font_atlas();
        if (w, h) != self.font_size {
            let (tex, view) = create_font_texture(device, queue, pixels, w, h);
            self.font_tex = tex;
            self.font_view = view;
            self.font_size = (w, h);
            self.rebuild_all_bind_groups(device);
        } else {
            queue.write_texture(
                ::wgpu::TexelCopyTextureInfo {
                    texture: &self.font_tex,
                    mip_level: 0,
                    origin: ::wgpu::Origin3d::ZERO,
                    aspect: ::wgpu::TextureAspect::All,
                },
                pixels,
                ::wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w),
                    rows_per_image: Some(h),
                },
                ::wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    fn sync_color_sv_atlas(&mut self, device: &::wgpu::Device, queue: &::wgpu::Queue, ui: &mut Ui) {
        if !ui.color_sv_atlas_take_dirty() {
            return;
        }
        let (pixels, w, h) = ui.color_sv_atlas();
        self.set_texture_rgba(device, queue, TEX_SLOT_COLOR_SV, pixels, w, h, "color sv");
    }

    fn rebuild_all_bind_groups(&mut self, device: &::wgpu::Device) {
        self.default_bind_group = make_bind_group(
            device,
            &self.bind_layout,
            &self.uniform_buf,
            &self.font_view,
            &self.sampler,
            &self.placeholder_view,
        );
        for slot in self.tex_slots.values_mut() {
            slot.bind_group = make_bind_group(
                device,
                &self.bind_layout,
                &self.uniform_buf,
                &self.font_view,
                &self.sampler,
                &slot.view,
            );
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
        let c_tl = cmd.colors[0];
        let c_tr = cmd.colors[1];
        let c_br = cmd.colors[2];
        let c_bl = cmd.colors[3];
        let kind = cmd.kind;
        let tex = cmd.tex as f32;
        let params = cmd.params;
        out.extend_from_slice(&[
            UiVertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color: c_tl,
                kind,
                tex,
                params,
            },
            UiVertex {
                pos: [x1, y0],
                uv: [u1, v0],
                color: c_tr,
                kind,
                tex,
                params,
            },
            UiVertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color: c_br,
                kind,
                tex,
                params,
            },
            UiVertex {
                pos: [x0, y0],
                uv: [u0, v0],
                color: c_tl,
                kind,
                tex,
                params,
            },
            UiVertex {
                pos: [x1, y1],
                uv: [u1, v1],
                color: c_br,
                kind,
                tex,
                params,
            },
            UiVertex {
                pos: [x0, y1],
                uv: [u0, v1],
                color: c_bl,
                kind,
                tex,
                params,
            },
        ]);
    }
    out
}

fn create_font_texture(
    device: &::wgpu::Device,
    queue: &::wgpu::Queue,
    pixels: &[u8],
    w: u32,
    h: u32,
) -> (::wgpu::Texture, ::wgpu::TextureView) {
    let texture = device.create_texture(&::wgpu::TextureDescriptor {
        label: Some("mega-ui font atlas"),
        size: ::wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: ::wgpu::TextureDimension::D2,
        format: ::wgpu::TextureFormat::R8Unorm,
        usage: ::wgpu::TextureUsages::TEXTURE_BINDING | ::wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        ::wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: ::wgpu::Origin3d::ZERO,
            aspect: ::wgpu::TextureAspect::All,
        },
        pixels,
        ::wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w.max(1)),
            rows_per_image: Some(h.max(1)),
        },
        ::wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&::wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_rgba_texture(
    device: &::wgpu::Device,
    queue: &::wgpu::Queue,
    pixels: &[u8],
    w: u32,
    h: u32,
    label: &str,
) -> (::wgpu::Texture, ::wgpu::TextureView) {
    let w = w.max(1);
    let h = h.max(1);
    let texture = device.create_texture(&::wgpu::TextureDescriptor {
        label: Some(label),
        size: ::wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: ::wgpu::TextureDimension::D2,
        format: ::wgpu::TextureFormat::Rgba8Unorm,
        usage: ::wgpu::TextureUsages::TEXTURE_BINDING | ::wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    if !pixels.is_empty() {
        queue.write_texture(
            ::wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: ::wgpu::Origin3d::ZERO,
                aspect: ::wgpu::TextureAspect::All,
            },
            pixels,
            ::wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            ::wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
    let view = texture.create_view(&::wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_placeholder_rgba(
    device: &::wgpu::Device,
    queue: &::wgpu::Queue,
) -> (::wgpu::Texture, ::wgpu::TextureView) {
    create_rgba_texture(
        device,
        queue,
        &[180, 200, 255, 255],
        1,
        1,
        "mega-ui tex0 placeholder",
    )
}

fn make_bind_group(
    device: &::wgpu::Device,
    layout: &::wgpu::BindGroupLayout,
    uniform_buf: &::wgpu::Buffer,
    font_view: &::wgpu::TextureView,
    sampler: &::wgpu::Sampler,
    tex0_view: &::wgpu::TextureView,
) -> ::wgpu::BindGroup {
    device.create_bind_group(&::wgpu::BindGroupDescriptor {
        label: Some("mega-ui bind group"),
        layout,
        entries: &[
            ::wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            ::wgpu::BindGroupEntry {
                binding: 1,
                resource: ::wgpu::BindingResource::TextureView(font_view),
            },
            ::wgpu::BindGroupEntry {
                binding: 2,
                resource: ::wgpu::BindingResource::Sampler(sampler),
            },
            ::wgpu::BindGroupEntry {
                binding: 3,
                resource: ::wgpu::BindingResource::TextureView(tex0_view),
            },
        ],
    })
}

fn draw_batched(pass: &mut ::wgpu::RenderPass<'_>, renderer: &UiRenderer, cmds: &[DrawCommand]) {
    if cmds.is_empty() {
        return;
    }

    let bind_for = |slot: Option<u32>| -> &::wgpu::BindGroup {
        match slot.and_then(|s| renderer.tex_slots.get(&s)) {
            Some(tex) => &tex.bind_group,
            None => &renderer.default_bind_group,
        }
    };

    let mut batch_start = 0usize;
    let mut bound_slot: Option<u32> = None;

    let flush = |pass: &mut ::wgpu::RenderPass<'_>, start: usize, end: usize, slot: Option<u32>| {
        if end <= start {
            return;
        }
        pass.set_bind_group(0, bind_for(slot), &[]);
        let v0 = (start * 6) as u32;
        let v1 = (end * 6) as u32;
        pass.draw(v0..v1, 0..1);
    };

    for (i, cmd) in cmds.iter().enumerate() {
        if cmd.kind >= 0.5 && cmd.kind < 1.5 {
            let slot = Some(cmd.tex);
            if slot != bound_slot {
                flush(pass, batch_start, i, bound_slot);
                batch_start = i;
                bound_slot = slot;
            }
        }
    }
    flush(pass, batch_start, cmds.len(), bound_slot);
}

fn count_draw_stats(cmds: &[DrawCommand]) -> DrawStats {
    let quads = cmds.len().min(MAX_QUADS);
    let slice = &cmds[..quads];
    let mut batches = 0usize;
    let mut batch_start = 0usize;
    let mut bound_slot: Option<u32> = None;
    for (i, cmd) in slice.iter().enumerate() {
        if cmd.kind >= 0.5 && cmd.kind < 1.5 {
            let slot = Some(cmd.tex);
            if slot != bound_slot {
                if i > batch_start {
                    batches += 1;
                }
                batch_start = i;
                bound_slot = slot;
            }
        }
    }
    if quads > batch_start {
        batches += 1;
    }
    DrawStats {
        commands: cmds.len(),
        batches,
        quads,
    }
}
