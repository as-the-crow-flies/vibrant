use color;
use gaussian;
use occlusion;
use pbr;
use random;
use uniforms;

@group(0) @binding(0) var RADIANCE: texture_3d<f32>;
@group(0) @binding(1) var GAUSSIAN: texture_3d<f32>;
@group(0) @binding(3) var WEIGHTS: texture_3d<f32>;
@group(0) @binding(7) var<uniform> CASCADE_OPTS: CascadeOpts;
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

    // Lobes live in world space, so shading does too.
    let view = -normalize((ENVIRONMENT.camera.transform * vec4<f32>(far - near, 0.0)).xyz);

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
        let noise = step_noise(fragment.position.xy, t);
        let outgoing_radiance = sample_outgoing_radiance(uv, noise, normal, view, albedo, F, Fdr);

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

// The gradient is per voxel; a normal maps to world by the transpose of the
// world → voxel transform.
fn sample_normal(sample: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let raw_normal = - 2.0 * textureSampleLevel(GRADIENT, SAMPLER, sample, 0.0).xyz + 1.0;
    if (length(raw_normal) <= 1E-4) { return fallback; }

    let voxels = vec3<f32>(textureDimensions(GRADIENT)) * raw_normal;
    return normalize(transpose(linear_part(TRANSFORM)) * voxels);
}

// Per pixel, step and frame. `tent` in [-1, 1]³ jitters the probe lookup:
// averaged by accumulation, trilinear interpolation becomes a smooth B-spline.
// `box` in [-½, ½)³ turns a nearest-probe read into trilinear on average, and
// `xi` in [0, 1) picks the lobe.
struct StepNoise {
    tent: vec3<f32>,
    box: vec3<f32>,
    xi: f32,
}

fn step_noise(pixel: vec2<f32>, t: f32) -> StepNoise {
    let a = pcg3d(vec3<u32>(vec2<u32>(pixel), bitcast<u32>(ENVIRONMENT.time) ^ bitcast<u32>(t)));
    let b = pcg3d(a);
    let c = pcg3d(b);

    return StepNoise(
        (vec3<f32>(a) + vec3<f32>(b)) * 2.3283064365386963e-10 - 1.0,
        vec3<f32>(c) * 2.3283064365386963e-10 - 0.5,
        f32(pcg3d(c).x) * 2.3283064365386963e-10,
    );
}

fn sample_outgoing_radiance(
    uv: vec3<f32>,
    noise: StepNoise,
    normal: vec3<f32>,
    view: vec3<f32>,
    albedo: vec3<f32>,
    F: f32,
    Fdr: f32) -> vec3<f32>
{
    let anisotropy = HDRI_SETTINGS.anisotropy;

    let shading = Shading(
        specular_kernel(normal, view, HDRI_SETTINGS.roughness),
        phase_kernel(view, anisotropy),
        abs(anisotropy) < 1e-3,
    );

    let offset = HDRI_SETTINGS.lookup_jitter * noise.tent;

    var radiance = Radiance();

    if (HDRI_SETTINGS.lobe_sampling == 0u) {
        radiance = all_lobes(probe_grid_at(GAUSSIAN, uv, offset), shading);
    } else if (HDRI_SETTINGS.nearest_probes != 0u) {
        radiance = nearest_lobe(nearest_probe(uv, offset + noise.box), noise.xi, shading);
    } else {
        radiance = filtered_lobe(probe_grid_at(GAUSSIAN, uv, offset), noise.xi, shading);
    }

    return radiance.specular * F + HDRI_SETTINGS.phase * radiance.phase * albedo * (1.0 - Fdr);
}

// The kernels the light lobes are integrated against. Isotropic phase is
// precomputed per probe by cascade 0.
struct Shading {
    specular: Kernel,
    phase: Kernel,
    isotropic: bool,
}

struct Radiance {
    specular: vec3<f32>,
    phase: vec3<f32>,
}

fn all_lobes(grid: ProbeGrid, shading: Shading) -> Radiance {
    let lobes = CASCADE_OPTS.vmm_size;

    var specular = vec3<f32>(0.0);
    var phase = vec3<f32>(0.0);

    if (shading.isotropic) {
        phase = textureSampleLevel(PHASE, SAMPLER, grid.base / grid.stride, 0.0).rgb;

        for (var k = 0u; k < lobes; k += 4u) {
            let l0 = grid_lobe(grid, k);
            let l1 = grid_lobe(grid, k + 1u);
            let l2 = grid_lobe(grid, k + 2u);
            let l3 = grid_lobe(grid, k + 3u);
            specular += l0.phi * sg_inner_weight(l0, shading.specular) + l1.phi * sg_inner_weight(l1, shading.specular)
                + l2.phi * sg_inner_weight(l2, shading.specular) + l3.phi * sg_inner_weight(l3, shading.specular);
        }
    } else {
        for (var k = 0u; k < lobes; k += 4u) {
            let l0 = grid_lobe(grid, k);
            let l1 = grid_lobe(grid, k + 1u);
            let l2 = grid_lobe(grid, k + 2u);
            let l3 = grid_lobe(grid, k + 3u);
            specular += l0.phi * sg_inner_weight(l0, shading.specular) + l1.phi * sg_inner_weight(l1, shading.specular)
                + l2.phi * sg_inner_weight(l2, shading.specular) + l3.phi * sg_inner_weight(l3, shading.specular);
            phase += l0.phi * sg_inner_weight(l0, shading.phase) + l1.phi * sg_inner_weight(l1, shading.phase)
                + l2.phi * sg_inner_weight(l2, shading.phase) + l3.phi * sg_inner_weight(l3, shading.phase);
        }
    }

    return Radiance(specular, phase);
}

// One lobe of the probe nearest `uv` moved by `offset` cells, read unfiltered.
fn nearest_lobe(probe: vec3<i32>, xi: f32, shading: Shading) -> Radiance {
    let grid = vec3<i32>(probe_count());

    let pick = pick_lobe(WeightSource(true, probe, vec3<f32>(0.0)), xi);
    let texel = probe + grid * vec3<i32>(lobe_tile(pick.index));
    let lobe = sg_lobe(SG(textureLoad(GAUSSIAN, texel, 0), textureLoad(RADIANCE, texel, 0).rgb));

    var radiance = picked_radiance(lobe, pick, shading);
    if (shading.isotropic) { radiance.phase = textureLoad(PHASE, probe, 0).rgb; }

    return radiance;
}

// One lobe of the trilinearly filtered probe at `grid`.
fn filtered_lobe(grid: ProbeGrid, xi: f32, shading: Shading) -> Radiance {
    let local = grid.base / grid.stride;

    let pick = pick_lobe(WeightSource(false, vec3<i32>(0), local), xi);
    let lobe = grid_lobe(grid, pick.index);

    var radiance = picked_radiance(lobe, pick, shading);
    if (shading.isotropic) { radiance.phase = textureSampleLevel(PHASE, SAMPLER, local, 0.0).rgb; }

    return radiance;
}

struct Pick {
    index: u32,
    // Σw / w: one over the pick's probability.
    scale: f32,
}

// Where the weights are read: a probe's texels, or trilinearly at `local`.
struct WeightSource {
    nearest: bool,
    probe: vec3<i32>,
    local: vec3<f32>,
}

// The weights of lobes 4j..4j+3.
fn weight_texel(source: WeightSource, j: u32) -> vec4<f32> {
    if (source.nearest) {
        let grid = vec3<i32>(probe_count());
        return textureLoad(WEIGHTS, source.probe + grid * vec3<i32>(weight_tile(j)), 0);
    }

    return textureSampleLevel(WEIGHTS, SAMPLER, (vec3<f32>(weight_tile(j)) + source.local) / 2.0, 0.0);
}

// Lobe k with probability w_k / Σw; no lit lobe gives scale 0. Two passes over
// the texels, no arrays: dynamically indexed arrays spill out of registers.
fn pick_lobe(source: WeightSource, xi: f32) -> Pick {
    let texels = (CASCADE_OPTS.vmm_size + 3u) / 4u;

    var total = 0.0;
    for (var j = 0u; j < texels; j++) { total += dot(weight_texel(source, j), vec4<f32>(1.0)); }

    if (total <= 0.0) { return Pick(0u, 0.0); }

    let threshold = min(xi, 0.99999) * total;
    var cdf = 0.0;

    for (var j = 0u; j < texels; j++) {
        let w = weight_texel(source, j);
        let prefix = cdf + vec4<f32>(w.x, w.x + w.y, w.x + w.y + w.z, w.x + w.y + w.z + w.w);

        if (prefix.w > threshold) {
            // Lobes at or below the threshold, zero weights included, come first.
            let below = vec3<f32>(prefix.xyz <= vec3<f32>(threshold));
            let k = u32(below.x + below.y + below.z);
            let weight = dot(w, vec4<f32>(vec4<u32>(0u, 1u, 2u, 3u) == vec4<u32>(k)));
            return Pick(4u * j + k, total / weight);
        }

        cdf = prefix.w;
    }

    return Pick(0u, 0.0);
}

fn picked_radiance(lobe: Lobe, pick: Pick, shading: Shading) -> Radiance {
    let phi = pick.scale * lobe.phi;

    var radiance = Radiance(phi * sg_inner_weight(lobe, shading.specular), vec3<f32>(0.0));
    if (!shading.isotropic) { radiance.phase = phi * sg_inner_weight(lobe, shading.phase); }

    return radiance;
}

fn probe_count() -> vec3<u32> {
    return max(textureDimensions(GAUSSIAN) / vec3<u32>(4u, 4u, 2u), vec3<u32>(1u));
}

// Texel coordinates of the probe nearest `uv` moved by `offset` cells, clamped
// to the grid.
fn nearest_probe(uv: vec3<f32>, offset: vec3<f32>) -> vec3<i32> {
    let probes = vec3<f32>(probe_count());
    return vec3<i32>(clamp(floor(uv * probes + offset), vec3<f32>(0.0), probes - 1.0));
}

fn lobe_tile(k: u32) -> vec3<u32> {
    return vec3<u32>(k & 3u, (k >> 2u) & 3u, (k >> 4u) & 1u);
}

fn weight_tile(j: u32) -> vec3<u32> {
    return vec3<u32>(j & 1u, (j >> 1u) & 1u, (j >> 2u) & 1u);
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

fn grid_lobe(grid: ProbeGrid, k: u32) -> Lobe {
    return sg_lobe(probe_grid_sg(RADIANCE, GAUSSIAN, SAMPLER, grid, k));
}
