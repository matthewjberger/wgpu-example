struct Uniform {
    proj: mat4x4<f32>,
    proj_inv: mat4x4<f32>,
    view: mat4x4<f32>,
    cam_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_dir: vec3<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let tmp1 = i32(vertex_index) / 2;
    let tmp2 = i32(vertex_index) & 1;
    let pos = vec4<f32>(
        f32(tmp1) * 4.0 - 1.0,
        f32(tmp2) * 4.0 - 1.0,
        1.0,
        1.0
    );
    let inv_model_view = transpose(mat3x3<f32>(u.view[0].xyz, u.view[1].xyz, u.view[2].xyz));
    let unprojected = u.proj_inv * pos;
    var result: VertexOutput;
    result.world_dir = inv_model_view * unprojected.xyz;
    result.position = pos;
    return result;
}

@fragment
fn fs_sky(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_dir);

    let sky_top_color = vec3<f32>(0.385, 0.454, 0.55);
    let sky_horizon_color = vec3<f32>(0.646, 0.656, 0.67);
    let ground_horizon_color = vec3<f32>(0.646, 0.656, 0.67);
    let ground_bottom_color = vec3<f32>(0.2, 0.169, 0.133);

    let height = dir.y;

    let sky_curve = 0.15;
    let ground_curve = 0.02;

    var sky_color: vec3<f32>;

    if height > 0.0 {
        let t = 1.0 - pow(1.0 - height, 1.0 / sky_curve);
        sky_color = mix(sky_horizon_color, sky_top_color, clamp(t, 0.0, 1.0));
    } else {
        let t = 1.0 - pow(1.0 + height, 1.0 / ground_curve);
        sky_color = mix(ground_horizon_color, ground_bottom_color, clamp(t, 0.0, 1.0));
    }

    sky_color = sky_color * 1.3;

    let sun_direction = normalize(vec3<f32>(0.0, 0.5, -1.0));
    let sun_angle = acos(dot(dir, sun_direction));
    let sun_disk = 1.0 - smoothstep(0.0, 0.02, sun_angle);
    let sun_color = vec3<f32>(1.0, 0.95, 0.8);
    sky_color = mix(sky_color, sun_color, sun_disk * 0.5);

    return vec4<f32>(sky_color, 1.0);
}
