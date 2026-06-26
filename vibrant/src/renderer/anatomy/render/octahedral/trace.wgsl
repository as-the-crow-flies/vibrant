@group(0) @binding( 0) var IRRADIANCE: texture_3d<f32>;
@group(0) @binding( 1) var RADIANCE_0: texture_3d<f32>;
@group(0) @binding( 2) var RADIANCE_1: texture_3d<f32>;
@group(0) @binding( 3) var RADIANCE_2: texture_3d<f32>;
@group(0) @binding( 4) var RADIANCE_3: texture_3d<f32>;
@group(0) @binding( 5) var RADIANCE_4: texture_3d<f32>;
@group(0) @binding( 6) var RADIANCE_5: texture_3d<f32>;
@group(0) @binding( 7) var RADIANCE_6: texture_3d<f32>;
@group(0) @binding( 8) var RADIANCE_7: texture_3d<f32>;
@group(0) @binding( 9) var RADIANCE_8: texture_3d<f32>;
@group(0) @binding(10) var RADIANCE_9: texture_3d<f32>;
@group(0) @binding(11) var TRANSMISSION_0: texture_3d<f32>;
@group(0) @binding(12) var TRANSMISSION_1: texture_3d<f32>;
@group(0) @binding(13) var TRANSMISSION_2: texture_3d<f32>;
@group(0) @binding(14) var TRANSMISSION_3: texture_3d<f32>;
@group(0) @binding(15) var TRANSMISSION_4: texture_3d<f32>;
@group(0) @binding(16) var TRANSMISSION_5: texture_3d<f32>;
@group(0) @binding(17) var TRANSMISSION_6: texture_3d<f32>;
@group(0) @binding(18) var TRANSMISSION_7: texture_3d<f32>;
@group(0) @binding(19) var TRANSMISSION_8: texture_3d<f32>;
@group(0) @binding(20) var TRANSMISSION_9: texture_3d<f32>;

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

    let pixel = vec2<u32>(fragment.position.xy + 1000.00 * ENVIRONMENT.time);

    var t0 = max(hit.x, 0.0) + hash(fragment.position.xy + fract(ENVIRONMENT.time));
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
        let gradient_norm = select(vec3<f32>(0.0), gradient.xyz / gradient.a, gradient.a > 0.01);

        let sample_light = sample - 0.05 * gradient.xyz;

        let n_dot_v  = max(dot(gradient_norm, view), 0.0001);
        let roughness = HDRI_SETTINGS.roughness;
        let f0        = vec3<f32>(0.04);

        let F = gradient.a * F_Schlick(n_dot_v, f0) * HDRI_SETTINGS.specular;

        let albedo = material.scattering / max(material.extinction, vec3<f32>(0.001));


        let diffuse  = (vec3<f32>(1.0) - F) * albedo * sample_diffuse(sample_light);

        var out_scattering = diffuse;

        if (HDRI_SETTINGS.specular > 0.5) {
            let specular = sample_specular(pixel, sample_light, gradient.xyz, view, roughness);
            out_scattering = (vec3<f32>(1.0) - F) * albedo * specular;
        }

        color += transmittance * transmittance_in_step * out_scattering;

        transmittance *= exp(-extinction);

        if (all(transmittance <= vec3<f32>(1E-2))) { break; }
    }

    let alpha = 1.0 - brightness(transmittance);

    return vec4<f32>(linear_to_srgb(aces(color.rgb)), alpha);
}

fn sample_diffuse(uv: vec3<f32>) -> vec3<f32> {
    return tex(IRRADIANCE, uv); // Incorrect, should adjust for integer division
}

struct Material {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    extinction: vec3<f32>,
};

fn sample_material(sample: vec3<f32>) -> Material {
    let absorption = tex(ABSORPTION, sample);
    let scattering = tex(SCATTERING, sample);
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
    position: vec3<f32>,
    gradient: vec3<f32>,
    view: vec3<f32>,
    roughness: f32) -> vec3<f32> {
    const N: u32 = 1u;

    let offset = hash22(pixel);

    var result = vec3<f32>(0.0);

    for (var i = 0u; i < N; i++) {
        // var xi = hammersley_rotated(i, N, offset);
        var xi = offset;
        var pdf = 1.0;
        var dir = vec2<u32>(0u);

        var transmission = vec3<f32>(1.0);
        var radiance = vec3<f32>(0.0);

        for (var level: u32 = 0u; level < 10u; level++) {
            let i00 = sample_radiance_transmission(position, dir + vec2<u32>(0u, 0u), level);
            let i10 = sample_radiance_transmission(position, dir + vec2<u32>(1u, 0u), level);
            let i01 = sample_radiance_transmission(position, dir + vec2<u32>(0u, 1u), level);
            let i11 = sample_radiance_transmission(position, dir + vec2<u32>(1u, 1u), level);

            let cx = route(xi.x, i00.w + i01.w, i10.w + i11.w);
            let cy = route(xi.y, select(i00.w, i10.w, cx.c), select(i01.w, i11.w, cx.c));

            xi = vec2<f32>(cx.r, cy.r);
            pdf *= cx.p * cy.p;
            dir = 2u * (dir + vec2<u32>(u32(cx.c), u32(cy.c)));

            let r = select(select(i00.radiance, i01.radiance, cy.c), select(i10.radiance, i11.radiance, cy.c), cx.c);
            let t = select(select(i00.transmission, i01.transmission, cy.c), select(i10.transmission, i11.transmission, cy.c), cx.c);

            radiance += transmission * r;
            transmission *= t;
        }

        result += radiance;
    }

    return result / f32(N);
}

struct RadianceTransmission {
    radiance: vec3<f32>,
    transmission: vec3<f32>,
    w: f32
}

fn sample_radiance_transmission(
    uv: vec3<f32>,
    offset: vec2<u32>,
    level: u32
) -> RadianceTransmission {
    let probes = max(textureDimensions(IRRADIANCE) >> vec3<u32>(level), vec3<u32>(1u));

    let voxel = uv * vec3<f32>(textureDimensions(IRRADIANCE)) / f32(1u << level);

    let sample = vec3<f32>(vec3<u32>(offset, 0u) * probes) +
        clamp(voxel, vec3<f32>(0.5), vec3<f32>(probes) - 0.5);

    switch level {
        case  0: { return radiance_transmission(RADIANCE_0, TRANSMISSION_0, sample); }
        case  1: { return radiance_transmission(RADIANCE_1, TRANSMISSION_1, sample); }
        case  2: { return radiance_transmission(RADIANCE_2, TRANSMISSION_2, sample); }
        case  3: { return radiance_transmission(RADIANCE_3, TRANSMISSION_3, sample); }
        case  4: { return radiance_transmission(RADIANCE_4, TRANSMISSION_4, sample); }
        case  5: { return radiance_transmission(RADIANCE_5, TRANSMISSION_5, sample); }
        case  6: { return radiance_transmission(RADIANCE_6, TRANSMISSION_6, sample); }
        case  7: { return radiance_transmission(RADIANCE_7, TRANSMISSION_7, sample); }
        case  8: { return radiance_transmission(RADIANCE_8, TRANSMISSION_8, sample); }
        default: { return radiance_transmission(RADIANCE_9, TRANSMISSION_9, sample); }
    }
}

fn radiance_transmission(radiance: texture_3d<f32>, transmission: texture_3d<f32>, sample: vec3<f32>) -> RadianceTransmission {
    let s = sample / vec3<f32>(textureDimensions(radiance));

    let r = tex(radiance, s);
    let t = tex(transmission, s);

    return RadianceTransmission(r, t, brightness(r));
}

fn brightness(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

struct Split {
  c: bool,   // 0 -> first child, 1 -> second child
  r: f32,   // the random number rescaled back into [0,1)
  p: f32,   // probability of the branch taken (for the pdf product)
}

// Pick between two masses a, b using r in [0,1); rescale r into the chosen half.
fn route(r: f32, a: f32, b: f32) -> Split {
    let total = a + b;

    if (total <= 0.0) { return Split(false, 0.0, 1.0); }

    let pa = select(a / total, 0.5, HDRI_SETTINGS.specular > 0.7);

    if (r < pa) { return Split(false, r / pa, pa); }
    else { return Split(true, (r - pa) / (1.0 - pa), 1.0 - pa); }
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

fn tex(tex: texture_3d<f32>, uv: vec3<f32>) -> vec3<f32> {
    return unpack_rgb(textureSampleLevel(tex, SAMPLER, uv, 0.0));
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}
