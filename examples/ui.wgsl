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
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) kind: f32,
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
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    if v.kind < 0.5 {
        let a = textureSampleLevel(font_atlas, font_sampler, v.uv, 0.0).r;
        return vec4(v.color.rgb, v.color.a * a);
    }
    // Host rebinds `tex0` between batches according to DrawCommand.tex.
    let sample = textureSampleLevel(tex0, font_sampler, v.uv, 0.0);
    return sample * v.color;
}
