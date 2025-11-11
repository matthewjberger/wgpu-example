struct Uniform {
    view_proj: mat4x4<f32>,
    camera_world_pos: vec3<f32>,
    grid_size: f32,
    grid_min_pixels: f32,
    grid_cell_size: f32,
    orthographic_scale: f32,
    is_orthographic: f32,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = vec3<f32>(0.0);

    switch vertex_index {
        case 0u: { pos = vec3<f32>(-10.0, 0.0, -10.0); }
        case 1u: { pos = vec3<f32>(10.0, 0.0, -10.0); }
        case 2u: { pos = vec3<f32>(-10.0, 0.0, 10.0); }
        case 3u: { pos = vec3<f32>(-10.0, 0.0, 10.0); }
        case 4u: { pos = vec3<f32>(10.0, 0.0, -10.0); }
        case 5u: { pos = vec3<f32>(10.0, 0.0, 10.0); }
        default: {}
    }

    let grid_scale = select(1.0, max(10.0, ubo.orthographic_scale * 100.0), ubo.is_orthographic > 0.5);
    pos = pos * ubo.grid_size * grid_scale;
    let world_pos = vec3<f32>(
        pos.x + ubo.camera_world_pos.x,
        0.0,
        pos.z + ubo.camera_world_pos.z
    );

    var output: VertexOutput;
    var clip_pos = ubo.view_proj * vec4<f32>(world_pos, 1.0);

    if (ubo.is_orthographic > 0.5) {
        clip_pos.z = clamp(clip_pos.z, 0.0, clip_pos.w);
    }

    output.clip_position = clip_pos;
    output.world_pos = world_pos;
    return output;
}

fn mod_pos(pos: f32, size: f32) -> f32 {
    return pos - size * floor(pos / size);
}


@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dvx = vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.x));
    let dvy = vec2<f32>(dpdx(in.world_pos.z), dpdy(in.world_pos.z));
    let lx = length(dvx);
    let ly = length(dvy);
    let dudv = vec2<f32>(lx, ly);
    let l = length(dudv);

    let effective_scale = select(l, l * ubo.orthographic_scale, ubo.orthographic_scale > 1.0);
    let lod = max(0.0, log10(effective_scale * ubo.grid_min_pixels / ubo.grid_cell_size) + 1.0);
    let cell_size_lod0 = ubo.grid_cell_size * pow(10.0, floor(lod));
    let cell_size_lod1 = cell_size_lod0 * 10.0;
    let cell_size_lod2 = cell_size_lod1 * 10.0;

    let dudv4 = dudv * 8.0;

    let mod_lod0 = vec2<f32>(
        mod_pos(in.world_pos.x, cell_size_lod0),
        mod_pos(in.world_pos.z, cell_size_lod0)
    ) / dudv4;
    let lod0_alpha = max2(vec2<f32>(1.0) - abs(saturate(mod_lod0) * 2.0 - vec2<f32>(1.0)));

    let mod_lod1 = vec2<f32>(
        mod_pos(in.world_pos.x, cell_size_lod1),
        mod_pos(in.world_pos.z, cell_size_lod1)
    ) / dudv4;
    let lod1_alpha = max2(vec2<f32>(1.0) - abs(saturate(mod_lod1) * 2.0 - vec2<f32>(1.0)));

    let mod_lod2 = vec2<f32>(
        mod_pos(in.world_pos.x, cell_size_lod2),
        mod_pos(in.world_pos.z, cell_size_lod2)
    ) / dudv4;
    let lod2_alpha = max2(vec2<f32>(1.0) - abs(saturate(mod_lod2) * 2.0 - vec2<f32>(1.0)));

    let lod_fade = fract(lod);

    let grid_color_thin = vec4<f32>(0.75, 0.75, 0.75, 0.25);
    let grid_color_thick = vec4<f32>(0.2, 0.4, 0.8, 0.4);

    var color: vec4<f32>;
    if (lod2_alpha > 0.0) {
        color = grid_color_thick;
        color.a *= lod2_alpha * 0.7;
    } else if (lod1_alpha > 0.0) {
        let fade = smoothstep(0.2, 0.8, lod_fade);
        color = mix(grid_color_thick, grid_color_thin, fade);
        color.a *= lod1_alpha * 0.5;
    } else {
        color = grid_color_thin;
        color.a *= (lod0_alpha * (1.0 - lod_fade)) * 0.4;
    }

    if (ubo.is_orthographic < 0.5) {
        let dist = length(in.world_pos.xz - ubo.camera_world_pos.xz);
        let opacity_falloff = 1.0 - smoothstep(0.8 * ubo.grid_size, ubo.grid_size * 3.0, dist);
        color.a *= opacity_falloff;
    }

    let x_axis_nearby = abs(in.world_pos.z) < 0.03;
    let z_axis_nearby = abs(in.world_pos.x) < 0.03;

    if (x_axis_nearby) {
        color = mix(color, vec4<f32>(0.87, 0.26, 0.24, 0.7), 0.5);
    }
    if (z_axis_nearby) {
        color = mix(color, vec4<f32>(0.24, 0.7, 0.29, 0.7), 0.5);
    }

    if (color.a < 0.02) {
        discard;
    }

    return color;
}


fn log10(x: f32) -> f32 {
    return log2(x) / log2(10.0);
}

fn saturate(x: vec2<f32>) -> vec2<f32> {
    return clamp(x, vec2<f32>(0.0), vec2<f32>(1.0));
}

fn saturate_f32(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn max2(v: vec2<f32>) -> f32 {
    return max(v.x, v.y);
}
