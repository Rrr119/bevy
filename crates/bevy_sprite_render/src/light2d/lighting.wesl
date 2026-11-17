const MAX_LIGHTS_2D: u32 = 32u;
const MAX_LIGHT_OCCLUDER_SEGMENTS_2D: u32 = 128u;

struct GpuPointLight2d {
    // xy: position, z: range, w: intensity
    position_range_intensity: vec4<f32>,
    // rgb: color, a: reserved
    color: vec4<f32>,
    // x: height, y: shadows enabled
    params: vec4<f32>,
}

struct Lighting2d {
    // rgb: ambient color * brightness
    ambient: vec4<f32>,
    // x: light count, y: occluder segment count
    counts: vec4<u32>,
    lights: array<GpuPointLight2d, MAX_LIGHTS_2D>,
    occluders: array<vec4<f32>, MAX_LIGHT_OCCLUDER_SEGMENTS_2D>,
}

@group(0) @binding(4) var<uniform> lighting_2d: Lighting2d;

fn flat_normal_2d() -> vec3<f32> {
    return vec3<f32>(0.0, 0.0, 1.0);
}

fn tangent_space_normal_to_world_2d(
    tangent_space_normal: vec3<f32>,
    tangent: vec2<f32>,
    bitangent: vec2<f32>,
) -> vec3<f32> {
    var t = tangent;
    var b = bitangent;
    if dot(t, t) < 0.0000001 {
        t = vec2<f32>(1.0, 0.0);
    } else {
        t = normalize(t);
    }
    if dot(b, b) < 0.0000001 {
        b = vec2<f32>(0.0, 1.0);
    } else {
        b = normalize(b);
    }

    let n = normalize(tangent_space_normal);
    return normalize(vec3<f32>(t * n.x + b * n.y, max(n.z, 0.001)));
}

fn sample_normal_map_2d(sample: vec4<f32>) -> vec3<f32> {
    return sample.rgb * 2.0 - 1.0;
}

fn shadow_visibility_2d(world_position: vec2<f32>, light_position: vec2<f32>) -> f32 {
    let to_light = light_position - world_position;
    let distance_to_light = length(to_light);
    if distance_to_light <= 0.0001 {
        return 1.0;
    }

    let ray_direction = to_light / distance_to_light;
    let segment_count = min(lighting_2d.counts.y, MAX_LIGHT_OCCLUDER_SEGMENTS_2D);

    for (var i = 0u; i < segment_count; i += 1u) {
        let segment = lighting_2d.occluders[i];
        let a = segment.xy;
        let b = segment.zw;
        let segment_direction = b - a;
        let denominator = ray_direction.x * segment_direction.y
            - ray_direction.y * segment_direction.x;

        if abs(denominator) <= 0.00001 {
            continue;
        }

        let offset = a - world_position;
        let hit_distance = (offset.x * segment_direction.y
            - offset.y * segment_direction.x) / denominator;
        let segment_t = (offset.x * ray_direction.y
            - offset.y * ray_direction.x) / denominator;

        if hit_distance > 0.0001
            && hit_distance < distance_to_light
            && segment_t >= 0.0
            && segment_t <= 1.0 {
            return 0.0;
        }
    }

    return 1.0;
}

fn evaluate_point_light_2d(
    world_position: vec2<f32>,
    normal: vec3<f32>,
    light_index: u32,
) -> vec3<f32> {
    let light = lighting_2d.lights[light_index];
    let to_light = light.position_range_intensity.xy - world_position;
    let range = light.position_range_intensity.z;
    if range <= 0.0 {
        return vec3<f32>(0.0);
    }

    let distance_to_light = length(to_light);
    if distance_to_light >= range {
        return vec3<f32>(0.0);
    }

    let falloff = 1.0 - distance_to_light / range;
    let attenuation = falloff * falloff;
    let height = light.params.x;
    let light_direction = normalize(vec3<f32>(to_light, height));
    let diffuse = max(dot(normal, light_direction), 0.0);
    var visibility = 1.0;
    if light.params.y > 0.5 {
        visibility = shadow_visibility_2d(
            world_position,
            light.position_range_intensity.xy,
        );
    }

    return light.color.rgb
        * light.position_range_intensity.w
        * attenuation
        * diffuse
        * visibility;
}

fn apply_lighting_2d(
    world_position: vec2<f32>,
    normal: vec3<f32>,
    albedo: vec4<f32>,
) -> vec4<f32> {
    var irradiance = lighting_2d.ambient.rgb;
    let light_count = min(lighting_2d.counts.x, MAX_LIGHTS_2D);
    for (var i = 0u; i < light_count; i += 1u) {
        irradiance += evaluate_point_light_2d(world_position, normal, i);
    }
    return vec4<f32>(albedo.rgb * irradiance, albedo.a);
}
