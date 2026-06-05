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

    while (t0 < t1) {
        let sample = origin + direction * (t0 + 2.0);

        if (tex(GRADIENT, sample).a > 0.0) { break; }
        else { t0 += 2.0; }
    }

    for (var t = t0; t < t1; t += step) {
        let sample = origin + direction * t;

        let material = sample_material(sample);
        let extinction = step * material.extinction;

        if (any(extinction < vec3<f32>(1E-5))) { continue; }

        let gradient = sample_gradient(sample);
        let gradient_norm = select(vec3<f32>(0.0), gradient.xyz / gradient.a, gradient.a > 0.01);

        let diffuse = sample_diffuse(sample);
        let diffuse_sample = diffuse * material.scattering;

        let reflection = normalize(reflect(direction_norm, gradient_norm));
        let specular = vec3<f32>(0.0);

        let transmittance_in_step = 1.0 - exp(-extinction);

        color += transmittance * transmittance_in_step * (diffuse_sample + specular);

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
