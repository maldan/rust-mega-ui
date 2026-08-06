struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
}

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) kind: f32,
    @location(4) tex: f32,
    @location(5) params: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) kind: f32,
    @location(3) params: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var font_atlas: texture_2d<f32>;
@group(0) @binding(2) var font_sampler: sampler;
@group(0) @binding(3) var tex0: texture_2d<f32>;

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let ndc = vec2(
        (v.pos.x / uniforms.viewport.x) * 2.0 - 1.0,
        1.0 - (v.pos.y / uniforms.viewport.y) * 2.0,
    );
    out.clip = vec4(ndc, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    out.kind = v.kind;
    out.params = v.params;
    return out;
}

// Inigo Quilez rounded box; `b` = half-size, `r` = corner radii
// (x=TR, y=BR, z=TL, w=BL) in a Y-down UI frame after centering.
fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    var rr = r;
    // p.x>0 → (TR, BR); else (TL, BL)
    rr = select(rr.zwxy, rr.xyzw, p.x > 0.0);
    // Y-down: p.y>0 is bottom → use .y (BR/BL); else top → .x (TR/TL)
    let radius = select(rr.x, rr.y, p.y > 0.0);
    let q = abs(p) - b + vec2(radius);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - radius;
}

fn sdf_round_fill(uv: vec2<f32>, params: vec4<f32>, color: vec4<f32>) -> vec4<f32> {
    let size = max(params.xy, vec2(1e-3));
    let radius = params.z;
    let mode = params.w; // 0=all, 1=top, 2=bot
    var corners = vec4(radius);
    if mode > 1.5 {
        // bottom only: TR,BR,TL,BL
        corners = vec4(0.0, radius, 0.0, radius);
    } else if mode > 0.5 {
        // top only
        corners = vec4(radius, 0.0, radius, 0.0);
    }
    let half_size = size * 0.5;
    // uv (0,0)=top-left of content rect → centered p, Y down.
    let p = uv * size - half_size;
    let d = sd_rounded_box(p, half_size, corners);
    // Analytic AA in screen space.
    let aa = fwidth(d) * 0.5;
    let a = 1.0 - smoothstep(-aa, aa, d);
    return vec4(color.rgb, color.a * a);
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    if v.kind < 0.5 {
        let a = textureSampleLevel(font_atlas, font_sampler, v.uv, 0.0).r;
        return vec4(v.color.rgb, v.color.a * a);
    }
    if v.kind > 1.5 {
        return sdf_round_fill(v.uv, v.params, v.color);
    }
    // Host rebinds `tex0` between batches according to DrawCommand.tex.
    let sample = textureSampleLevel(tex0, font_sampler, v.uv, 0.0);
    return sample * v.color;
}
