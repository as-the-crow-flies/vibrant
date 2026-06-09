@group(0) @binding( 0) var IRRADIANCE: texture_3d<f32>;
@group(0) @binding( 1) var RADIANCE_0: texture_3d<f32>;
@group(0) @binding( 2) var RADIANCE_1: texture_3d<f32>;
@group(0) @binding( 3) var RADIANCE_2: texture_3d<f32>;
@group(0) @binding( 4) var RADIANCE_3: texture_3d<f32>;
@group(0) @binding( 5) var RADIANCE_4: texture_3d<f32>;
@group(0) @binding( 6) var RADIANCE_5: texture_3d<f32>;
@group(0) @binding( 7) var TRANSMISSION_0: texture_3d<f32>;
@group(0) @binding( 8) var TRANSMISSION_1: texture_3d<f32>;
@group(0) @binding( 9) var TRANSMISSION_2: texture_3d<f32>;
@group(0) @binding(10) var TRANSMISSION_3: texture_3d<f32>;
@group(0) @binding(11) var TRANSMISSION_4: texture_3d<f32>;
@group(0) @binding(12) var TRANSMISSION_5: texture_3d<f32>;

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

    var t0 = max(hit.x, 0.0) + hash(fragment.position.xy + fract(ENVIRONMENT.time));
    let t1 = hit.y;

    var transmittance = vec3<f32>(1.0);
    var color = vec3<f32>(0.0);

    var step = 0.5;

    let direction_norm = normalize(direction);
    let view = -direction_norm;

    while (t0 < t1) {
        let sample = origin + direction * (t0 + 2.0);

        if (tex(GRADIENT, sample).a > 0.0) { break; }
        else { t0 += 2.0; }
    }

    for (var t = t0; t < t1; t += step) {
        let sample = origin + direction * t;

        let material = sample_material(sample);
        let extinction = step * material.extinction;

        if (all(extinction < vec3<f32>(1E-5))) { continue; }

        let transmittance_in_step = 1.0 - exp(-extinction);

        let gradient = sample_gradient(sample);
        let gradient_norm = select(vec3<f32>(0.0), gradient.xyz / gradient.a, gradient.a > 0.01);

        let sample_light = sample - 0.05 * gradient.xyz;

        let n_dot_v  = max(dot(gradient_norm, view), 0.0001);
        let roughness = HDRI_SETTINGS.roughness;
        let f0        = vec3<f32>(0.04);

        let F = gradient.a * F_Schlick(n_dot_v, f0) * HDRI_SETTINGS.specular;

        let albedo = material.scattering; // / max(material.extinction, vec3<f32>(0.001));

        let diffuse  = (vec3<f32>(1.0) - F) * albedo * sample_diffuse(sample_light);
        let specular = F *  sample_specular(sample_light, gradient_norm, view, roughness);

        color += transmittance * transmittance_in_step * (diffuse + specular);

        transmittance *= exp(-extinction);

        if (all(transmittance <= vec3<f32>(1E-5))) { break; }
    }

    let alpha = 1.0 - dot(transmittance, vec3<f32>(1.0 / 3.0));

    return vec4<f32>(color.rgb, alpha);
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
    return unpack_rgb(tex(IRRADIANCE, uv));
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

fn sample_radiance_cascade(position: vec3<f32>, octant: vec3<u32>, sub5: vec2<u32>, n: u32) -> vec3<f32> {
    let irr_dim = textureDimensions(IRRADIANCE);
    let probes = irr_dim >> vec3<u32>(n);
    let subdivisions = 1u << n;
    let probe = min(vec3<u32>(position * vec3<f32>(probes)), probes - vec3<u32>(1u));
    let sub = sub5 >> vec2<u32>(5u - n);
    let voxel = vec3<u32>(
        octant.x * subdivisions * probes.x + sub.x * probes.x + probe.x,
        octant.y * subdivisions * probes.y + sub.y * probes.y + probe.y,
        octant.z * probes.z + probe.z
    );
    let uv = vec3<f32>(
        (f32(voxel.x) + 0.5) / f32(2u * irr_dim.x),
        (f32(voxel.y) + 0.5) / f32(2u * irr_dim.y),
        (f32(voxel.z) + 0.5) / f32(2u * probes.z)
    );
    switch (n) {
        case 0u:  { return unpack_rgb(tex(RADIANCE_0, uv)); }
        case 1u:  { return unpack_rgb(tex(RADIANCE_1, uv)); }
        case 2u:  { return unpack_rgb(tex(RADIANCE_2, uv)); }
        case 3u:  { return unpack_rgb(tex(RADIANCE_3, uv)); }
        case 4u:  { return unpack_rgb(tex(RADIANCE_4, uv)); }
        default:  { return unpack_rgb(tex(RADIANCE_5, uv)); }
    }
}

fn sample_specular(position: vec3<f32>, normal: vec3<f32>, view: vec3<f32>, roughness: f32) -> vec3<f32> {
    if (dot(normal, normal) < 0.5) { return vec3<f32>(0.0); }

    let n     = min(u32(clamp(1.0 - roughness, 0.0, 1.0) * 5.0), 5u);
    let count = 1u << n;
    let shift = 5u - n;  // sub5 → level-n sub index

    let mirror = normalize(reflect(-view, normal));
    let oct    = vec3<u32>(u32(mirror.x < 0.0), u32(mirror.y < 0.0), u32(mirror.z < 0.0));
    let sub5   = octahedron_inverse(mirror, 32u);

    var transmission = vec3<f32>(1.0);
    if (n > 0u) { transmission *= cascade_transmission(TRANSMISSION_0, position, oct, sub5, 0u); }
    if (n > 1u) { transmission *= cascade_transmission(TRANSMISSION_1, position, oct, sub5, 1u); }
    if (n > 2u) { transmission *= cascade_transmission(TRANSMISSION_2, position, oct, sub5, 2u); }
    if (n > 3u) { transmission *= cascade_transmission(TRANSMISSION_3, position, oct, sub5, 3u); }
    if (n > 4u) { transmission *= cascade_transmission(TRANSMISSION_4, position, oct, sub5, 4u); }

    return transmission * sample_radiance_cascade(position, oct, sub5, n);
}

fn cascade_transmission(t: texture_3d<f32>, position: vec3<f32>, octant: vec3<u32>, sub5: vec2<u32>, c: u32) -> vec3<f32> {
    let probes = textureDimensions(IRRADIANCE) >> vec3<u32>(c);
    let subdivisions = 1u << c;
    let probe = min(vec3<u32>(position * vec3<f32>(probes)), probes - vec3<u32>(1u));
    let sub = sub5 >> vec2<u32>(5u - c);
    let voxel = vec3<u32>(
        octant.x * subdivisions * probes.x + sub.x * probes.x + probe.x,
        octant.y * subdivisions * probes.y + sub.y * probes.y + probe.y,
        octant.z * probes.z + probe.z
    );
    return unpack_rgb(tex(t, (vec3<f32>(voxel) + 0.5) / vec3<f32>(textureDimensions(t))));
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

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}
