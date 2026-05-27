@group(0) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING: texture_3d<f32>;
@group(0) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(0) @binding(3) var GRADIENT: texture_3d<f32>;
@group(0) @binding(4) var SAMPLER: sampler;
@group(0) @binding(5) var<uniform> TRANSFORM: mat4x4<f32>;
@group(0) @binding(6) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(1) @binding( 0) var RADIANCE_0: texture_3d<f32>;
@group(1) @binding( 1) var RADIANCE_1: texture_3d<f32>;
@group(1) @binding( 2) var RADIANCE_2: texture_3d<f32>;
@group(1) @binding( 3) var RADIANCE_3: texture_3d<f32>;
@group(1) @binding( 4) var RADIANCE_4: texture_3d<f32>;
@group(1) @binding( 5) var RADIANCE_5: texture_3d<f32>;
@group(1) @binding( 6) var TRANSMISSION_0: texture_3d<f32>;
@group(1) @binding( 7) var TRANSMISSION_1: texture_3d<f32>;
@group(1) @binding( 8) var TRANSMISSION_2: texture_3d<f32>;
@group(1) @binding( 9) var TRANSMISSION_3: texture_3d<f32>;
@group(1) @binding(10) var TRANSMISSION_4: texture_3d<f32>;
@group(1) @binding(11) var TRANSMISSION_5: texture_3d<f32>;
@group(1) @binding(12) var CASCADE_SAMPLER: sampler;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Fragment {
    let uv = vec2<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2))
    );

    return Fragment(vec4<f32>(uv, 0.0, 1.0), uv);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let near = unproject(vec3<f32>(fragment.uv, 0.0));
    let far = unproject(vec3<f32>(fragment.uv, 1.0));

    let direction_world = normalize(far - near);

    let origin = (TRANSFORM * vec4<f32>(near, 1.0)).xyz;
    let direction = (TRANSFORM * vec4<f32>(direction_world, 0.0)).xyz;

    let hit = intersectAABB(origin, direction);

    if (hit.x > hit.y) {
        return vec4<f32>(aces(sample_hdri(direction_world)), 0.0);
    }

    var t0 = max(hit.x, 0.0) + hash(fragment.position.xy + fract(ENVIRONMENT.time));
    let t1 = hit.y;

    let light_direction = (TRANSFORM * vec4<f32>(ENVIRONMENT.light, 0.0)).xyz;

    var transmittance = vec3<f32>(1.0);
    var color = vec3<f32>(0.0);

    let phase_function = 1.0 / (4.0 * PI);

    var step = 0.5;

    let direction_norm = normalize(direction);

    while (t0 < t1) {
        let sample = origin + direction * (t0 + 4.0);

        if (tex(GRADIENT, sample).a > 0.0) { break; }
        else { t0 += 4.0; }
    }

    for (var t = t0; t < t1; t += step) {
        let sample = origin + direction * t;

        let material = sample_material(sample);
        let extinction = step * material.extinction;

        if (any(extinction < vec3<f32>(1E-5))) { continue; }

        let gradient = sample_gradient(sample);
        let gradient_norm = select(vec3<f32>(0.0), gradient.xyz / gradient.a, gradient.a > 0.01);

        // let light_sample = sample - 0.005 * gradient_norm;
        let light_sample = sample;

        let diffuse = sample_diffuse(light_sample);
        let diffuse_sample = diffuse * material.scattering * phase_function;

        let reflection = normalize(reflect(direction_norm, gradient_norm));
        let specular = gradient.a * HDRI_SETTINGS.strength * HDRI_SETTINGS.specular * sample_specular(light_sample, reflection);

        let transmittance_in_step = 1.0 - exp(-extinction);

        color += transmittance * transmittance_in_step * (diffuse_sample + specular);

        transmittance *= exp(-extinction);

        if (all(transmittance <= vec3<f32>(1E-5))) { break; }
    }

    let alpha = 1.0 - dot(transmittance, vec3<f32>(1.0 / 3.0));

    return vec4<f32>(aces(sample_hdri(direction_world) * transmittance + color.rgb), alpha);
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return saturate((x * (a * x + b)) / (x * (c * x + d) + e));
}

fn sample_diffuse(uv: vec3<f32>) -> vec3<f32> {
    var diffuse = vec3<f32>(0.0);

    for (var face=0u; face<6u; face++) {
        let sample = uv * vec3<f32>(1.0, 1.0, 0.1666666666666) +
            vec3<f32>(f32(face)) * vec3<f32>(0.0, 0.0, 0.1666666667);

        diffuse += tex(RADIANCE_0, sample).rgb;
    }

    return 4.0 * PI * diffuse;
}

fn sample_hdri(direction: vec3<f32>) -> vec3<f32> {
    if (HDRI_SETTINGS.show == 0) { return vec3<f32>(0.0); }

    let sample = equirectangular(direction, HDRI_SETTINGS.rotation);
    return HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, 0.0).rgb;
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

struct Material {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    extinction: vec3<f32>,
};

fn sample_material(sample: vec3<f32>) -> Material {
    let absorption = unpack_rgb(tex(ABSORPTION, sample));
    let scattering = unpack_rgb(tex(SCATTERING, sample));
    let extinction = absorption + scattering;

    return Material(absorption, scattering, extinction);
}

fn sample_gradient(sample: vec3<f32>) -> vec4<f32> {
    let gradient_raw = tex(GRADIENT, sample);
    let alpha = 2.0 * gradient_raw.a;
    return vec4<f32>(normalize(2.0 * gradient_raw.xyz - 1.0) * alpha, alpha);
}

fn sample_specular(uv: vec3<f32>, direction: vec3<f32>) -> vec3<f32> {
    let coordinate = cubemap_encode(direction);

    let max_level = 5u;

    var transmission = vec3<f32>(1.0);

    for (var level = 1u; level < max_level; level++) {
        transmission *= sample_transmission(uv, coordinate, level).rgb;
    }

    return transmission * sample_radiance(uv, coordinate, max_level).rgb;
}

fn cascade_sample(
    uv: vec3<f32>,
    coordinate: CubeCoordinates,
    level: u32
) -> vec3<f32> {

    let root_dim = level_dim(0) / vec3<u32>(1,1,6);
    let cascade_dim = level_dim(level) / vec3<u32>(1,1,6);
    let direction_dim = root_dim >> vec3<u32>(level);

    let n = 1u << level;

    let direction_index = min(
        vec2<u32>(coordinate.uv * f32(n)),
        vec2<u32>(n - 1u)
    );

    let face_offset = vec3<u32>(0,0,coordinate.face * cascade_dim.z);
    let tile_origin = vec3<u32>(direction_index * direction_dim.xy, 0);

    let voxel = clamp(
        uv * vec3<f32>(direction_dim),
        vec3<f32>(0.5),
        vec3<f32>(direction_dim) - vec3<f32>(0.5)
    );

    return (vec3<f32>(face_offset + tile_origin) + voxel) / vec3<f32>(level_dim(level));
}

fn level_dim(level: u32) -> vec3<u32> {
    switch level {
        case 0  : { return textureDimensions(RADIANCE_0); }
        case 1  : { return textureDimensions(RADIANCE_1); }
        case 2  : { return textureDimensions(RADIANCE_2); }
        case 3  : { return textureDimensions(RADIANCE_3); }
        case 4  : { return textureDimensions(RADIANCE_4); }
        default : { return textureDimensions(RADIANCE_5); }
    }
}

fn sample_transmission(uv: vec3<f32>, coordinate: CubeCoordinates, level: u32) -> vec3<f32> {
    let sample = cascade_sample(uv, coordinate, level);

    switch level {
        case 0u : { return tex(TRANSMISSION_0, sample).rgb; }
        case 1u : { return tex(TRANSMISSION_1, sample).rgb; }
        case 2u : { return tex(TRANSMISSION_2, sample).rgb; }
        case 3u : { return tex(TRANSMISSION_3, sample).rgb; }
        case 4u : { return tex(TRANSMISSION_4, sample).rgb; }
        default : { return tex(TRANSMISSION_5, sample).rgb; }
    }
}

fn sample_radiance(uv: vec3<f32>, coordinate: CubeCoordinates, level: u32) -> vec3<f32> {
    let sample = cascade_sample(uv, coordinate, level);

    switch level {
        case 0u : { return tex(RADIANCE_0, sample).rgb; }
        case 1u : { return tex(RADIANCE_1, sample).rgb; }
        case 2u : { return tex(RADIANCE_2, sample).rgb; }
        case 3u : { return tex(RADIANCE_3, sample).rgb; }
        case 4u : { return tex(RADIANCE_4, sample).rgb; }
        default : { return tex(RADIANCE_5, sample).rgb; }
    }
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}

fn intersectAABB(origin: vec3<f32>, direction: vec3<f32>) -> vec2<f32> {
    let boxMin = vec3<f32>(0.0);
    let boxMax = vec3<f32>(1.0);

    let invDir = 1.0 / direction;

    let t1 = (boxMin - origin) * invDir;
    let t2 = (boxMax - origin) * invDir;

    let tMinVec = min(t1, t2);
    let tMaxVec = max(t1, t2);

    let tMin = max(max(tMinVec.x, tMinVec.y), tMinVec.z);
    let tMax = min(min(tMaxVec.x, tMaxVec.y), tMaxVec.z);

    return vec2<f32>(tMin, tMax);
}

fn tex(tex: texture_3d<f32>, uv: vec3<f32>) -> vec4<f32> {
    return textureSampleLevel(tex, SAMPLER, uv, 0.0);
}
