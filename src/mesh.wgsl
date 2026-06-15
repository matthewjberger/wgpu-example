struct Uniform {
    mvp: array<mat4x4<f32>, 2>,
};

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex_main(vert: VertexInput, @builtin(view_index) view: u32) -> VertexOutput {
    var out: VertexOutput;
    out.color = vert.color;
    out.position = ubo.mvp[view] * vert.position;
    return out;
}

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
}
