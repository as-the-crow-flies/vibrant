@group(0) @binding( 1) var IRRADIANCE: texture_3d<f32>;
@group(0) @binding( 2) var RADIANCE_0: texture_3d<f32>;
@group(0) @binding( 3) var RADIANCE_1: texture_3d<f32>;
@group(0) @binding( 4) var RADIANCE_2: texture_3d<f32>;
@group(0) @binding( 5) var RADIANCE_3: texture_3d<f32>;
@group(0) @binding( 6) var RADIANCE_4: texture_3d<f32>;
@group(0) @binding( 7) var RADIANCE_5: texture_3d<f32>;
@group(0) @binding( 8) var RADIANCE_6: texture_3d<f32>;
@group(0) @binding( 9) var RADIANCE_7: texture_3d<f32>;
@group(0) @binding(10) var RADIANCE_8: texture_3d<f32>;
@group(0) @binding(11) var TRANSMISSION_0: texture_3d<f32>;
@group(0) @binding(12) var TRANSMISSION_1: texture_3d<f32>;
@group(0) @binding(13) var TRANSMISSION_2: texture_3d<f32>;
@group(0) @binding(14) var TRANSMISSION_3: texture_3d<f32>;
@group(0) @binding(15) var TRANSMISSION_4: texture_3d<f32>;
@group(0) @binding(16) var TRANSMISSION_5: texture_3d<f32>;
@group(0) @binding(17) var TRANSMISSION_6: texture_3d<f32>;
@group(0) @binding(18) var TRANSMISSION_7: texture_3d<f32>;
@group(0) @binding(19) var TRANSMISSION_8: texture_3d<f32>;
@group(0) @binding(20) var IMPORTANCE_0: texture_3d<f32>;
@group(0) @binding(21) var IMPORTANCE_1: texture_3d<f32>;
@group(0) @binding(22) var IMPORTANCE_2: texture_3d<f32>;
@group(0) @binding(23) var IMPORTANCE_3: texture_3d<f32>;
@group(0) @binding(24) var IMPORTANCE_4: texture_3d<f32>;
@group(0) @binding(25) var IMPORTANCE_5: texture_3d<f32>;
@group(0) @binding(26) var IMPORTANCE_6: texture_3d<f32>;
@group(0) @binding(27) var IMPORTANCE_7: texture_3d<f32>;
@group(0) @binding(28) var IMPORTANCE_8: texture_3d<f32>;

@group(1) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(1) @binding(1) var SCATTERING: texture_3d<f32>;
@group(1) @binding(2) var PROPERTIES: texture_3d<f32>;
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

        let n_dot_v  = max(dot(normal, -view), 0.0001);
        let roughness = HDRI_SETTINGS.roughness;
        let f0        = vec3<f32>(0.04);

        let F = F_Schlick(n_dot_v, f0) * HDRI_SETTINGS.specular;

        let albedo = material.scattering / max(material.extinction, vec3<f32>(0.001));
        let diffuse  = albedo * sample_diffuse(sample_light);
        let specular = sample_specular(pixel, sample_light, normal, view, roughness, f0);

        color += transmittance * transmittance_in_step * (
            (1.0 - F) * diffuse +
            F * specular
        );

        // color += transmittance * transmittance_in_step * specular;

        transmittance *= exp(-extinction);

        if (all(transmittance <= vec3<f32>(1E-2))) { break; }
    }

    let alpha = 1.0 - brightness(transmittance);

    return vec4<f32>(linear_to_srgb(aces(color.rgb)), alpha);
}

fn sample_diffuse(uv: vec3<f32>) -> vec3<f32> {
    return tex_rgb(IRRADIANCE, uv); // Incorrect, should adjust for integer division
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

fn sample_specular(
    pixel: vec2<u32>,
    uv: vec3<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    roughness: f32,
    f0: vec3<f32>) -> vec3<f32> {

    let N = 1u;

    var result = vec3<f32>(0.0);

    let hash = hash22(pixel);

    for (var i=0u; i<N; i++) {
        let random = hammersley_rotated(i, N, hash);
        let importance = sample_importance(random, uv, normal, view, roughness);
        let radiance = sample_radiance(uv, importance.uv, roughness);

        let light = octahedron_decode(2.0 * importance.uv - 1.0);

        let n_dot_l = max(dot(normal, light), 0.0);
        let brdf = GGX(normal, view, light, roughness, f0);

        result += radiance * brdf * n_dot_l * 4.0 * PI / importance.pdf;
    }

    return result / f32(N);
}

struct ImportanceSample {
    uv: vec2<f32>,
    pdf: f32
}

fn sample_importance(
    random: vec2<f32>,
    uv: vec3<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    roughness: f32
    ) -> ImportanceSample {
    var w = Warp(vec2<u32>(0u), random, f32(512 * 512));

    w = warp(w, importance(IMPORTANCE_0, 0, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_1, 1, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_2, 2, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_3, 3, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_4, 4, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_5, 5, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_6, 6, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_7, 7, uv, w.route, normal, view, roughness));
    w = warp(w, importance(IMPORTANCE_8, 8, uv, w.route, normal, view, roughness));

    return ImportanceSample(
        (vec2<f32>(w.route) + w.random) / 512.0, // 2^9
        w.probability
    );
}

fn importance(
    importance_tex: texture_3d<f32>,
    level: u32,
    uv: vec3<f32>,
    route: vec2<u32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    roughness: f32
) -> vec4<f32> {
    let cascade = tex(importance_tex, cascade_sample(0, uv, route));
    let brdf = brdf_importance(level, route, normal, view, roughness);
    return cascade * brdf;
}

fn brdf_importance(
    level: u32,
    route: vec2<u32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    roughness: f32
) -> vec4<f32> {
    if (ENVIRONMENT.settings.alpha < 0.5) { return vec4<f32>(1.0); }

    let inv_res = 1.0 / f32(1u << (level + 1u));
    let base = vec2<f32>(2u * route);

    let uv00 = (base + vec2<f32>(0.5, 0.5)) * inv_res;
    let uv10 = (base + vec2<f32>(1.5, 0.5)) * inv_res;
    let uv01 = (base + vec2<f32>(0.5, 1.5)) * inv_res;
    let uv11 = (base + vec2<f32>(1.5, 1.5)) * inv_res;

    let a2 = roughness * roughness;

    return vec4<f32>(
        ggx_weight(normal, view, octahedron_decode(2.0 * uv00 - 1.0), a2),
        ggx_weight(normal, view, octahedron_decode(2.0 * uv10 - 1.0), a2),
        ggx_weight(normal, view, octahedron_decode(2.0 * uv01 - 1.0), a2),
        ggx_weight(normal, view, octahedron_decode(2.0 * uv11 - 1.0), a2)
    );
}

fn sample_radiance(uv: vec3<f32>, direction: vec2<f32>, roughness: f32) -> vec3<f32> {
    let transmission = vec3<f32>(1.0) *
        tex_rgb(TRANSMISSION_0, cascade_sample(0, uv, vec2<u32>(direction *   1.0))) *
        tex_rgb(TRANSMISSION_1, cascade_sample(1, uv, vec2<u32>(direction *   2.0))) *
        tex_rgb(TRANSMISSION_2, cascade_sample(2, uv, vec2<u32>(direction *   4.0))) *
        tex_rgb(TRANSMISSION_3, cascade_sample(3, uv, vec2<u32>(direction *   8.0))) *
        tex_rgb(TRANSMISSION_4, cascade_sample(4, uv, vec2<u32>(direction *  16.0))) *
        tex_rgb(TRANSMISSION_5, cascade_sample(5, uv, vec2<u32>(direction *  32.0))) *
        tex_rgb(TRANSMISSION_6, cascade_sample(6, uv, vec2<u32>(direction *  64.0))) *
        tex_rgb(TRANSMISSION_7, cascade_sample(7, uv, vec2<u32>(direction * 128.0)));

    let radiance = tex_rgb(RADIANCE_8, cascade_sample(8, uv, vec2<u32>(direction * 256.0)));

    return transmission * radiance;
}

fn cascade_sample(level: u32, uv: vec3<f32>, offset: vec2<u32>) -> vec3<f32> {
    let probes = vec3<f32>(max(textureDimensions(IRRADIANCE) >> vec3<u32>(level), vec3<u32>(1u)));
    let samples = vec3<f32>(vec3<u32>(vec2<u32>(1u << level), 1u));

    let probe = uv * vec3<f32>(textureDimensions(IRRADIANCE)) / f32(1u << level);
    let probe_clamped = clamp(probe, vec3<f32>(0.5), vec3<f32>(probes) - 0.5);

    let sample = vec3<f32>(vec3<u32>(offset, 0u));

    return (sample * probes + probe_clamped) / (probes * samples);
}

struct Warp {
    route: vec2<u32>,
    random: vec2<f32>,
    probability: f32
}

struct Split {
    route: bool,
    random: f32,
    probability: f32,
}

fn warp(warp: Warp, i: vec4<f32>) -> Warp {
    let sx = route(
        warp.random.x,
        i.x + i.z, // Left
        i.y + i.w  // Right
    );

    let sy = route(
        warp.random.y,
        select(i.x, i.y, sx.route), // Top
        select(i.z, i.w, sx.route) // Bottom
    );

    return Warp(
        2u * warp.route + vec2<u32>(u32(sx.route), u32(sy.route)),
        vec2<f32>(sx.random, sy.random) / vec2<f32>(sx.probability, sy.probability),
        warp.probability * sx.probability * sy.probability
    );
}

fn route(random: f32, a: f32, b: f32) -> Split {
    let pa = a / max(a + b, 1E-5);
    let route = random >= pa;

    return Split(
        route,
        select(random, random - pa, route),
        select(pa, 1.0 - pa, route)
    );
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
