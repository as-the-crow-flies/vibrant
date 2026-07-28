@group(0) @binding(0) var RADIANCE: texture_3d<f32>;
@group(0) @binding(1) var GAUSSIAN: texture_3d<f32>;

@group(1) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(1) @binding(1) var SCATTERING: texture_3d<f32>;
@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var GRADIENT: texture_3d<f32>;
@group(1) @binding(4) var SAMPLER: sampler;
@group(1) @binding(5) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(6) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

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

    if (hit.x > hit.y) { return vec4<f32>(0.0); }

    let pixel = vec2<u32>(fragment.position.xy + 4096.0 * fract(ENVIRONMENT.time));

    var t0 = max(hit.x, 0.0) + hash(fragment.position.xy + 4096.0 * fract(ENVIRONMENT.time));
    let t1 = hit.y;

    var transmittance = vec3<f32>(1.0);
    var color = vec3<f32>(0.0);

    var step = 0.5;

    let direction_norm = normalize(direction);
    let view = -direction_norm;

    while (t0 < t1) {
        let sample = origin + direction * (t0 + 2.0);

        if (textureSampleLevel(GRADIENT, SAMPLER, sample, 0.0).a > 0.0) { break; }
        else { t0 += 2.0; }
    }

    for (var t = t0; t < t1; t += step) {
        let sample = origin + direction * t;

        let material = sample_material(sample);
        let extinction = step * material.extinction;

        if (all(extinction < vec3<f32>(1E-5))) { continue; }

        let transmittance_in_step = 1.0 - exp(-extinction);

        let gradient = sample_gradient(sample);
        let normal = -select(vec3<f32>(0.0), gradient.xyz / gradient.a, gradient.a > 0.01);

        let sample_light = sample - 0.05 * gradient.xyz;

        let albedo = material.scattering / max(material.extinction, vec3<f32>(0.001));

        color += 0.1 * transmittance * transmittance_in_step * albedo * sample_irradiance(sample_light, normal, 5u);

        transmittance *= exp(-extinction);

        if (all(transmittance <= vec3<f32>(1E-2))) { break; }
    }

    let alpha = 1.0 - brightness(transmittance);

    return vec4<f32>(linear_to_srgb(aces(color.rgb)), alpha);
}

struct Material {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    extinction: vec3<f32>,
};

fn sample_material(sample: vec3<f32>) -> Material {
    let absorption = tex_rgb(ABSORPTION, sample);
    let scattering = tex_rgb(SCATTERING, sample);
    let extinction = absorption + scattering;

    return Material(absorption, scattering, extinction);
}

fn sample_gradient(sample: vec3<f32>) -> vec4<f32> {
    let gradient_raw = textureSampleLevel(GRADIENT, SAMPLER, sample, 0.0);
    let alpha = 2.0 * gradient_raw.a;
    return vec4<f32>(normalize(2.0 * gradient_raw.xyz - 1.0) * alpha, alpha);
}

fn sample_radiance(uv: vec3<f32>, direction: vec3<f32>, level: u32) -> vec3<f32> {
    var radiance = vec3<f32>(0.0);
    let lod = f32(level);
    var pi = 0.0;

    for (var k = 0u; k < VMM_SIZE; k++) {
        let octant = vec3<u32>(u32((k & 1u) != 0u), u32((k & 2u) != 0u), u32((k & 4u) != 0u));
        let octant_min = vec3<f32>(octant) * 0.5;

        let texel = 0.5 / vec3<f32>(textureDimensions(GAUSSIAN, i32(lod)));

        let lobe_uv = clamp((uv + vec3<f32>(octant)) * 0.5, octant_min + texel, octant_min + 0.5 - texel);

        let vmm = vmf(textureSampleLevel(GAUSSIAN, SAMPLER, lobe_uv, lod), direction);
        let phi = textureSampleLevel(RADIANCE, SAMPLER, lobe_uv, lod).rgb;

        radiance += phi * vmm;
        pi += vmm;
    }

    return radiance / max(pi, EPSILON);
}

fn sample_irradiance(uv: vec3<f32>, normal: vec3<f32>, level: u32) -> vec3<f32> {
    var irradiance = vec3<f32>(0.0);
    let lod = f32(level);

    for (var k = 0u; k < VMM_SIZE; k++) {
        let octant = vec3<u32>(u32((k & 1u) != 0u), u32((k & 2u) != 0u), u32((k & 4u) != 0u));
        let octant_min = vec3<f32>(octant) * 0.5;

        let texel = 0.5 / vec3<f32>(textureDimensions(GAUSSIAN, i32(lod)));

        let lobe_uv = clamp((uv + vec3<f32>(octant)) * 0.5, octant_min + texel, octant_min + 0.5 - texel);

        let v = textureSampleLevel(GAUSSIAN, SAMPLER, lobe_uv, lod);
        let phi = textureSampleLevel(RADIANCE, SAMPLER, lobe_uv, lod).rgb;

        irradiance += sg_irradiance_fitted(phi, v, normal);
    }

    return irradiance / PI;
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

fn tex_rgb(tex: texture_3d<f32>, uv: vec3<f32>) -> vec3<f32> {
    return unpack_rgb(textureSampleLevel(tex, SAMPLER, uv, 0.0));
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}
