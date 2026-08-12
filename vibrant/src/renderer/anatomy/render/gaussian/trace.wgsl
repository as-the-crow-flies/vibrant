@group(0) @binding(0) var RADIANCE: texture_3d<f32>;
@group(0) @binding(1) var GAUSSIAN: texture_3d<f32>;

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

        let reflection = reflect(-view, normal); // warped NDF axis

        let sample_light = sample - 0.05 * gradient.xyz;

        let n_dot_v = max(dot(normal, -view), 0.0001);
        let roughness = HDRI_SETTINGS.roughness;
        let f0 = vec3<f32>(0.04);

        let F = F_Schlick(n_dot_v, f0) * HDRI_SETTINGS.specular;

        let albedo = material.scattering / max(material.extinction, vec3<f32>(0.001));

        let lighting = sample_lighting(sample_light, normal, view, roughness, f0);

        let irradiance = (1.0 - F) * albedo * lighting.diffuse + F * lighting.specular;

        color += transmittance * transmittance_in_step
               * mix(0.05 * hdri(reflection, 4096), irradiance, ENVIRONMENT.settings.alpha);

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

struct Lighting {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
}

fn sample_lighting(uv: vec3<f32>, normal: vec3<f32>, view: vec3<f32>, roughness: f32, f0: vec3<f32>) -> Lighting {
    var diffuse = vec3<f32>(0.0);
    var specular = vec3<f32>(0.0);;

    let dims = vec3<f32>(textureDimensions(GAUSSIAN));
    let probes = max(floor(dims / vec3<f32>(4.0, 4.0, 2.0)), vec3<f32>(1.0));

    for (var k = 0u; k < VMM_SIZE; k++) {
        let tile = vec3<f32>(vec3<u32>(k & 3, (k >> 2) & 3, (k >> 4) & 1));

        let local = clamp(uv * probes, vec3<f32>(0.5), probes - 0.5);
        let lobe_uv = (tile * probes + local) / dims;

        let v = textureSampleLevel(GAUSSIAN, SAMPLER, lobe_uv, 0.0);
        let phi = textureSampleLevel(RADIANCE, SAMPLER, lobe_uv, 0.0).rgb;

        diffuse += sg_irradiance_fitted(phi, v, normal);
        specular += sg_specular_fitted(phi, v, view, normal, roughness, f0);
    }

    return Lighting(diffuse / PI, specular);
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
