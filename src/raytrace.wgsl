enable wgpu_ray_query;

struct CameraProperties {
    view_inverse: mat4x4<f32>,
    proj_inverse: mat4x4<f32>,
    light_pos: vec4<f32>,
    params: vec4<f32>,
    frame: vec4<u32>,
};

struct Vertex {
    pos_refl: vec4<f32>,
    normal: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var tlas: acceleration_structure;
@group(0) @binding(1) var out_image: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> cam: CameraProperties;
@group(0) @binding(3) var<storage, read> vertices: array<Vertex>;
@group(0) @binding(4) var<storage, read> indices: array<u32>;
@group(0) @binding(5) var<storage, read_write> accum: array<vec4<f32>>;

const IOR_R: f32 = 1.50;
const IOR_G: f32 = 1.52;
const IOR_B: f32 = 1.54;
const TWO_PI: f32 = 6.28318530718;

struct Hit {
    hit_pos: vec3<f32>,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    reflectivity: f32,
    mat_id: f32,
    t: f32,
    front_face: f32,
};

struct Basis {
    tangent: vec3<f32>,
    bitangent: vec3<f32>,
};

fn pcg(state: ptr<function, u32>) -> u32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rnd(state: ptr<function, u32>) -> f32 {
    return f32(pcg(state)) * (1.0 / 4294967296.0);
}

fn sky_color(direction: vec3<f32>) -> vec3<f32> {
    let height = 0.5 * (normalize(direction).y + 1.0);
    return mix(vec3<f32>(0.98, 0.86, 0.72), vec3<f32>(0.16, 0.24, 0.46), height);
}

fn glsl_mod(value: f32, modulus: f32) -> f32 {
    return value - modulus * floor(value / modulus);
}

fn checker(position: vec3<f32>) -> vec3<f32> {
    let scaled = position * 0.6;
    let sum = floor(scaled.x) + floor(scaled.z);
    return mix(vec3<f32>(0.90, 0.91, 0.94), vec3<f32>(0.05, 0.07, 0.11), glsl_mod(sum, 2.0));
}

fn make_basis(normal: vec3<f32>) -> Basis {
    var reference: vec3<f32>;
    if (abs(normal.x) > 0.9) {
        reference = vec3<f32>(0.0, 1.0, 0.0);
    } else {
        reference = vec3<f32>(1.0, 0.0, 0.0);
    }
    let tangent = normalize(cross(normal, reference));
    let bitangent = cross(normal, tangent);
    return Basis(tangent, bitangent);
}

fn sample_cone(direction: vec3<f32>, cos_theta_max: f32, seed: ptr<function, u32>) -> vec3<f32> {
    let u1 = rnd(seed);
    let u2 = rnd(seed);
    let cos_theta = mix(1.0, cos_theta_max, u1);
    let sin_theta = sqrt(max(0.0, 1.0 - cos_theta * cos_theta));
    let phi = TWO_PI * u2;
    let basis = make_basis(direction);
    return normalize(
        basis.tangent * (cos(phi) * sin_theta)
        + basis.bitangent * (sin(phi) * sin_theta)
        + direction * cos_theta
    );
}

fn trace_closest(origin: vec3<f32>, direction: vec3<f32>) -> Hit {
    var hit: Hit;
    var query: ray_query;
    let desc = RayDesc(RAY_FLAG_FORCE_OPAQUE, 0xFFu, 0.001, 10000.0, origin, direction);
    rayQueryInitialize(&query, tlas, desc);
    while (rayQueryProceed(&query)) {}
    let intersection = rayQueryGetCommittedIntersection(&query);

    if (intersection.kind == RAY_QUERY_INTERSECTION_NONE) {
        hit.t = -1.0;
        return hit;
    }

    let primitive = intersection.primitive_index;
    let index0 = indices[3u * primitive + 0u];
    let index1 = indices[3u * primitive + 1u];
    let index2 = indices[3u * primitive + 2u];
    let vertex0 = vertices[index0];
    let vertex1 = vertices[index1];
    let vertex2 = vertices[index2];

    let barycentrics = vec3<f32>(
        1.0 - intersection.barycentrics.x - intersection.barycentrics.y,
        intersection.barycentrics.x,
        intersection.barycentrics.y
    );

    var normal = normalize(
        vertex0.normal.xyz * barycentrics.x
        + vertex1.normal.xyz * barycentrics.y
        + vertex2.normal.xyz * barycentrics.z
    );
    let color = vertex0.color.xyz * barycentrics.x
        + vertex1.color.xyz * barycentrics.y
        + vertex2.color.xyz * barycentrics.z;

    let front = dot(normal, direction) < 0.0;
    if (!front) {
        normal = -normal;
    }

    hit.hit_pos = origin + direction * intersection.t;
    hit.normal = normal;
    hit.albedo = color;
    hit.reflectivity = vertex0.pos_refl.w;
    hit.mat_id = vertex0.normal.w;
    hit.t = intersection.t;
    if (front) {
        hit.front_face = 1.0;
    } else {
        hit.front_face = 0.0;
    }
    return hit;
}

fn trace_shadow(origin: vec3<f32>, direction: vec3<f32>, tmax: f32) -> f32 {
    var query: ray_query;
    let desc = RayDesc(
        RAY_FLAG_TERMINATE_ON_FIRST_HIT | RAY_FLAG_FORCE_OPAQUE,
        0xFFu,
        0.001,
        tmax,
        origin,
        direction
    );
    rayQueryInitialize(&query, tlas, desc);
    while (rayQueryProceed(&query)) {}
    let intersection = rayQueryGetCommittedIntersection(&query);
    if (intersection.kind == RAY_QUERY_INTERSECTION_NONE) {
        return 0.0;
    }
    return 1.0;
}

fn render_sample(seed: ptr<function, u32>, sample_index: i32, launch_id: vec2<u32>, launch_size: vec2<u32>) -> vec3<f32> {
    let jitter = vec2<f32>(rnd(seed), rnd(seed)) - 0.5;
    let pixel = vec2<f32>(launch_id) + 0.5 + jitter;
    let in_uv = pixel / vec2<f32>(launch_size);
    let ndc = in_uv * 2.0 - 1.0;

    var origin = (cam.view_inverse * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;
    let view_target = cam.proj_inverse * vec4<f32>(ndc.x, ndc.y, 1.0, 1.0);
    var direction = normalize((cam.view_inverse * vec4<f32>(normalize(view_target.xyz), 0.0)).xyz);

    let max_bounces = i32(cam.params.y);
    let intensity = cam.params.z;
    let eps = 0.001;

    var color = vec3<f32>(0.0);
    var throughput = vec3<f32>(1.0);

    let hero = sample_index % 3;
    var dispersed = false;

    for (var bounce = 0; bounce <= max_bounces; bounce = bounce + 1) {
        let hit = trace_closest(origin, direction);

        if (hit.t < 0.0) {
            color = color + throughput * sky_color(direction);
            break;
        }

        let normal = normalize(hit.normal);
        let mat = i32(hit.mat_id + 0.5);

        if (mat == 3) {
            color = color + throughput * hit.albedo * 3.0;
            break;
        }

        if (mat == 2) {
            var ior: f32;
            if (hero == 0) {
                ior = IOR_R;
            } else if (hero == 1) {
                ior = IOR_G;
            } else {
                ior = IOR_B;
            }
            var eta: f32;
            if (hit.front_face > 0.5) {
                eta = 1.0 / ior;
            } else {
                eta = ior;
            }

            let cos_incident = clamp(dot(-direction, normal), 0.0, 1.0);
            var r0 = (1.0 - 1.52) / (1.0 + 1.52);
            r0 = r0 * r0;
            let fresnel = r0 + (1.0 - r0) * pow(1.0 - cos_incident, 5.0);

            let refracted = refract(direction, normal, eta);
            let total_internal = dot(refracted, refracted) < 1e-6;

            if (total_internal || rnd(seed) < fresnel) {
                direction = reflect(direction, normal);
                origin = hit.hit_pos + normal * eps;
            } else {
                direction = normalize(refracted);
                origin = hit.hit_pos - normal * eps;
                if (!dispersed) {
                    var mask: vec3<f32>;
                    if (hero == 0) {
                        mask = vec3<f32>(1.0, 0.0, 0.0);
                    } else if (hero == 1) {
                        mask = vec3<f32>(0.0, 1.0, 0.0);
                    } else {
                        mask = vec3<f32>(0.0, 0.0, 1.0);
                    }
                    throughput = throughput * 3.0 * mask;
                    dispersed = true;
                }
            }
            throughput = throughput * vec3<f32>(0.98);
            continue;
        }

        var albedo: vec3<f32>;
        if (mat == 0) {
            albedo = checker(hit.hit_pos);
        } else {
            albedo = hit.albedo;
        }

        let to_light = cam.light_pos.xyz - hit.hit_pos;
        let distance = length(to_light);
        let light_dir = to_light / distance;
        let sin_max = clamp(cam.light_pos.w / distance, 0.0, 0.999);
        let cos_max = sqrt(1.0 - sin_max * sin_max);
        let tmax = distance - cam.light_pos.w * 1.001;

        let shadow_samples = 4;
        var diffuse_acc = 0.0;
        for (var shadow_index = 0; shadow_index < shadow_samples; shadow_index = shadow_index + 1) {
            let shadow_dir = sample_cone(light_dir, cos_max, seed);
            let shadowed = trace_shadow(hit.hit_pos + normal * eps, shadow_dir, tmax);
            diffuse_acc = diffuse_acc + max(dot(normal, shadow_dir), 0.0) * (1.0 - shadowed);
        }
        let diffuse = (diffuse_acc / f32(shadow_samples)) * intensity;
        let local = albedo * (0.12 + diffuse);

        let reflectivity = hit.reflectivity;
        color = color + throughput * local * (1.0 - reflectivity);

        if (reflectivity <= 0.001) {
            break;
        }
        throughput = throughput * reflectivity;
        direction = reflect(direction, normal);
        origin = hit.hit_pos + normal * eps;
    }
    return color;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(out_image);
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y) {
        return;
    }
    let launch_id = global_id.xy;

    let spp = max(1, i32(cam.params.w));
    var color = vec3<f32>(0.0);
    for (var sample_index = 0; sample_index < spp; sample_index = sample_index + 1) {
        var seed = (launch_id.x * 1973u
            + launch_id.y * 9277u
            + (cam.frame.y * u32(spp) + u32(sample_index)) * 26699u) | 1u;
        color = color + render_sample(&seed, sample_index, launch_id, dimensions);
    }
    color = color / f32(spp);

    let index = launch_id.y * dimensions.x + launch_id.x;
    var accumulated: vec3<f32>;
    if (cam.frame.x == 0u) {
        accumulated = color;
    } else {
        let previous = accum[index].xyz;
        accumulated = mix(previous, color, 1.0 / f32(cam.frame.x + 1u));
    }
    accum[index] = vec4<f32>(accumulated, 1.0);

    let mapped = pow(clamp(accumulated, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
    textureStore(out_image, vec2<i32>(launch_id), vec4<f32>(mapped, 1.0));
}
