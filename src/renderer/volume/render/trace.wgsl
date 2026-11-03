use color;
use gaussian;
use pbr;
use random;
use uniforms;

const VMM_SIZE: u32 = 32u;

@group(0) @binding(0) var RADIANCE: texture_3d<f32>;
@group(0) @binding(1) var GAUSSIAN: texture_3d<f32>;
@group(0) @binding(8) var PHASE: texture_3d<f32>;

@group(1) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(1) @binding(1) var SCATTERING: texture_3d<f32>;
@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var PROPERTIES: texture_3d<f32>;
@group(1) @binding(4) var GRADIENT: texture_3d<f32>;
@group(1) @binding(5) var SAMPLER: sampler;
@group(1) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(7) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

// The environment map itself is already integrated into the radiance cascade,
// so the volume tracer only needs the HDRI material settings. Binding 1 is the
// combined-mode line-depth target: the near→far fraction `s` of the first line
// hit, used to clamp this trace's far `t`.
@group(3) @binding(0) var<uniform> HDRI_SETTINGS: HdriSettings;
@group(3) @binding(1) var LINE_DEPTH: texture_2d<f32>;

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
    // View space: an imprecise `w` only slides a point along its own ray.
    let near = unproject(vec3<f32>(fragment.uv, 0.0));
    let far = unproject(vec3<f32>(fragment.uv, 1.0));

    let camera = TRANSFORM * ENVIRONMENT.camera.transform;
    let origin = (camera * vec4<f32>(near, 1.0)).xyz;
    let direction = (camera * vec4<f32>(normalize(far - near), 0.0)).xyz;

    let hit = intersectAABB(origin, direction);

    if (hit.x > hit.y) { return vec4<f32>(0.0); }

    // Combined mode: stop the march at the first line surface. The line tracer
    // writes `s`, the fraction of its near→far segment where it first hit; both
    // tracers share the same camera ray and volume TRANSFORM, so the matching
    // `t` here is `s * length(far - near)` (AABB `t` is arc length along
    // `direction`, whose fraction of the segment is `t / length(far - near)`).
    // LINE_DEPTH is cleared to 1.0, making this a no-op where no line was drawn.
    let line_s = textureLoad(LINE_DEPTH, vec2<i32>(fragment.position.xy), 0).r;

    let step = 0.5;
    let skip = 2.0;

    let entry = max(hit.x, 0.0);
    let t1 = min(hit.y, line_s * length(far - near));

    var transmittance = vec3<f32>(1.0);
    var color = vec3<f32>(0.0);

    let direction_norm = normalize(direction);
    let view = -direction_norm;

    // Coarse empty-space skip on an un-jittered `t0`, so the skipped distance
    // doesn't flicker frame to frame.
    var t0 = entry;
    while (t0 + skip < t1) {
        if (textureSampleLevel(GRADIENT, SAMPLER, origin + direction * (t0 + skip), 0.0).a > 0.0) { break; }
        t0 += skip;
    }

    // Step back one stride so the fine march can't miss a thin feature the
    // look-ahead jumped over, then jitter within a single fine step (not the
    // whole `[0, 1)` range, which is 2x `step` and just adds sampling variance).
    t0 = max(t0 - skip, entry) + step * hash(fragment.position.xy + 4096.0 * fract(ENVIRONMENT.time));

    var ior = 1.0;

    for (var t = t0; t < t1; t += step) {
        let uv = origin + direction * t;

        let material = sample_material(uv);

        // Fall back to `view` where the gradient is degenerate (a volume face
        // cut flat by the domain edge). A zero normal makes `half` below
        // `normalize(0)` -> NaN, which poisons the accumulation buffer and shows
        // up as a persistent noisy plane on that face.
        let normal = sample_normal(uv, view);
        let light = reflect(-view, normal);
        let half = normalize(view + light);

        let eta = material.ior / ior;
        ior = material.ior;

        let eta_boosted = mix(1.0, (eta - 1.0) * 5.0 + 1.0, HDRI_SETTINGS.specular);

        let F = fresnel(dot(view, half), eta_boosted);
        let Fdr = fresnel_diffuse(eta_boosted);

        let extinction = step * material.extinction;

        if (all(extinction < vec3<f32>(1E-5))) { continue; }

        let transmittance_in_step = 1.0 - exp(-extinction);

        let albedo = material.scattering / max(material.extinction, vec3<f32>(0.001));
        let outgoing_radiance = sample_outgoing_radiance(uv, normal, view, albedo, F, Fdr);

        color += transmittance * transmittance_in_step * outgoing_radiance;

        transmittance *= exp(-extinction);

        if (all(transmittance <= vec3<f32>(1E-2))) { break; }
    }

    let alpha = 1.0 - brightness(transmittance);

    // Linear HDR radiance; tone mapping happens once in the present pass.
    return vec4<f32>(color.rgb, alpha);
}

struct Material {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    extinction: vec3<f32>,
    ior: f32
};

fn sample_material(uv: vec3<f32>) -> Material {
    let absorption = tex_rgb(ABSORPTION, uv);
    let reduced_scattering = tex_rgb(SCATTERING, uv);
    let properties = tex_rgb(PROPERTIES, uv);

    let scattering = reduced_scattering / (1.0 - HDRI_SETTINGS.anisotropy);

    let extinction = absorption + scattering;
    let ior = average_ior(properties);

    return Material(absorption, scattering, extinction, ior);
}

fn sample_normal(sample: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let raw_normal = - 2.0 * textureSampleLevel(GRADIENT, SAMPLER, sample, 0.0).xyz + 1.0;
    let normal_len = length(raw_normal);
    return select(fallback, raw_normal / normal_len, normal_len > 1E-4);
}

fn sample_outgoing_radiance(
    uv: vec3<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    albedo: vec3<f32>,
    F: f32,
    Fdr: f32) -> vec3<f32>
{
    let roughness = HDRI_SETTINGS.roughness;
    let anisotropy = HDRI_SETTINGS.anisotropy;

    let specular = specular_kernel(normal, view, roughness);
    let phase = phase_kernel(view, anisotropy);
    let grid = probe_grid_at(GAUSSIAN, uv);

    var specular_radiance = vec3<f32>(0.0);
    var phase_radiance = vec3<f32>(0.0);

    // Isotropic phase: precomputed per probe by cascade 0.
    if (abs(anisotropy) < 1e-3) {
        phase_radiance = textureSampleLevel(PHASE, SAMPLER, grid.base / grid.stride, 0.0).rgb;

        for (var k = 0u; k < VMM_SIZE; k++) {
            let lobe = sg_lobe(probe_grid_sg(RADIANCE, GAUSSIAN, SAMPLER, grid, k));
            specular_radiance += lobe.phi * sg_inner_weight(lobe, specular);
        }
    } else {
        for (var k = 0u; k < VMM_SIZE; k++) {
            let lobe = sg_lobe(probe_grid_sg(RADIANCE, GAUSSIAN, SAMPLER, grid, k));

            specular_radiance += lobe.phi * sg_inner_weight(lobe, specular);
            phase_radiance += lobe.phi * sg_inner_weight(lobe, phase);
        }
    }

    return specular_radiance * F + phase_radiance * albedo * (1.0 - Fdr);
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
