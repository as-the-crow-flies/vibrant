use classify;
use color;
use environment;
use gaussian;
use microfacet;
use noise;
use occlusion;
use pbr;
use tracking;
use uniforms;

// Light each collision with a traced ray to the environment instead of the
// cascade: the cascade's ground truth.
const TRACED: bool = false;

// Traced: one Fresnel factor for the whole lobe, at the mirror direction, as
// the cascade. A const: as a uniform it made the traced shader 2× slower.
const MIRROR_FRESNEL: bool = false;

@group(0) @binding(0) var RADIANCE: texture_3d<f32>;
@group(0) @binding(1) var GAUSSIAN: texture_3d<f32>;
@group(0) @binding(3) var WEIGHTS: texture_3d<f32>;
@group(0) @binding(7) var<uniform> CASCADE_OPTS: CascadeOpts;
@group(0) @binding(8) var PHASE: texture_3d<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(2) @binding(1) var BLUE_NOISE: texture_3d<f32>;

// Binding 1 is the combined-mode line-depth target: the near→far fraction `s`
// of the first line hit, used to clamp this trace's far `t`. The rest is only
// read when TRACED: the environment and the lines as shadow-ray occluders.
@group(3) @binding(0) var<uniform> HDRI_SETTINGS: HdriSettings;
@group(3) @binding(1) var LINE_DEPTH: texture_2d<f32>;
@group(3) @binding(2) var ENVIRONMENT_MAP: texture_2d<f32>;
@group(3) @binding(3) var LINE_EXTINCTION: texture_3d<f32>;
@group(3) @binding(4) var LINE_SAMPLER: sampler;

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

    // Clipping to the crop box keeps the empty-space skip from marching it.
    let crop = CLASSIFY.crop;
    let hit = intersectAABB(origin, direction, max(0.5 + crop.min.xyz, vec3<f32>(0.0)), min(0.5 + crop.max.xyz, vec3<f32>(1.0)));

    if (hit.x > hit.y) { return vec4<f32>(0.0); }

    // Combined mode: stop the march at the first line surface. The line tracer
    // writes `s`, the fraction of its near→far segment where it first hit; both
    // tracers share the same camera ray and volume TRANSFORM, so the matching
    // `t` here is `s * length(far - near)` (AABB `t` is arc length along
    // `direction`, whose fraction of the segment is `t / length(far - near)`).
    // LINE_DEPTH is cleared to 1.0, making this a no-op where no line was drawn.
    let line_s = textureLoad(LINE_DEPTH, vec2<i32>(fragment.position.xy), 0).r;

    let entry = max(hit.x, 0.0);
    let exit = min(hit.y, line_s * length(far - near));

    // Lobes live in world space, so shading does too.
    let view = -normalize((ENVIRONMENT.camera.transform * vec4<f32>(far - near, 0.0)).xyz);

    let pixel = vec2<u32>(fragment.position.xy);

    // Both estimators share the tentative collisions: spectral delta tracking
    // (Kutz et al. 2017) picks the one real collision that's shaded, ratio
    // tracking estimates the transmittance behind alpha.
    var weight = vec3<f32>(1.0);
    var transmittance = vec3<f32>(1.0);
    var collided = false;
    // The real collision, shaded after the loop so neighbouring pixels shade
    // together instead of each at its own iteration.
    var collision = vec3<f32>(0.0);
    var collision_weight = vec3<f32>(0.0);

    var tracker = begin_tracking(origin, direction, entry, exit);

    // One draw per tentative collision: the optical depth to it carries across
    // block boundaries, so the first collision takes the pixel's blue noise.
    var i = 0u;
    var u = flight_noise(pixel, 2u, i);
    tracker.depth = -log(1.0 - u.x);

    // Measured fastest: flat for the cascade, nested when traced.
    while (tracking(tracker)) {
        if (TRACED) {
            if (!next_collision(&tracker)) { break; }
        } else if (!advance(&tracker)) {
            continue;
        }

        let majorant = tracker.majorant;
        let uv = origin + direction * tracker.t;
        let classified = classify(uv);
        let material = to_material(classified);
        let fictitious = max(majorant - material.extinction, vec3<f32>(0.0));

        if (!collided) {
            let real = average(weight * material.extinction);
            let total = real + average(weight * fictitious);
            let p = select(1.0, real / total, total > 0.0);

            if (u.y < p) {
                collision = uv;
                collision_weight = weight * material.extinction / (majorant * p);
                collided = true;
            } else {
                weight *= fictitious / (majorant * (1.0 - p));
            }
        }

        transmittance *= fictitious / majorant;

        // Russian roulette ends ratio tracking once little light gets through.
        let largest = max(transmittance.x, max(transmittance.y, transmittance.z));
        if (largest < ROULETTE) {
            let survival = largest / ROULETTE;
            transmittance = select(vec3<f32>(0.0), transmittance / survival, u.z < survival);
        }

        if (collided && all(transmittance == vec3<f32>(0.0))) { break; }

        i++;
        u = flight_noise(pixel, 2u, i);
        tracker.depth = -log(1.0 - u.x);
    }

    var color = vec3<f32>(0.0);

    if (collided) {
        let classified = classify(collision);
        let material = to_material(classified);

        if (TRACED) {
            color = collision_weight * traced_scatter(collision, direction, classified, material, view, pixel);
        } else {
            let lookup = noise(pixel, 0u);
            let pick = noise(pixel, 1u);
            let noise = StepNoise(tent(lookup.xyz), pick.xyz - 0.5, pick.w, lookup.w);
            color = collision_weight * scatter(collision, direction, classified, material, view, noise);
        }
    }

    let alpha = 1.0 - brightness(transmittance);

    // Linear HDR radiance; tone mapping happens once in the present pass.
    return vec4<f32>(color, alpha);
}

// The interface at a real collision. The ior one voxel back along the ray sets
// it; behind a cut (or outside the crop) there's none.
struct Interface {
    eta: f32,
    Fdr: f32,
    albedo: vec3<f32>,
}

fn interface_at(uv: vec3<f32>, direction: vec3<f32>, material: Material) -> Interface {
    let behind = classify(uv - normalize(direction) * voxel_step());
    let eta = select(material.ior / to_material(behind).ior, 1.0, behind.crop <= 0.0);
    let eta_boosted = mix(1.0, (eta - 1.0) * 2.0 + 1.0, HDRI_SETTINGS.specular);
    let albedo = material.scattering / max(material.extinction, vec3<f32>(0.001));

    return Interface(eta_boosted, fresnel_diffuse(eta_boosted), albedo);
}

// Outgoing radiance at a real collision from the cascade.
fn scatter(uv: vec3<f32>, direction: vec3<f32>, classified: Classified, material: Material, view: vec3<f32>, noise: StepNoise) -> vec3<f32> {
    let surface = interface_at(uv, direction, material);

    // Without an interface F is 0 whatever the normal: no gradient, no lobes.
    var normal = view;
    var specular = 0.0;

    if (surface.eta != 1.0) {
        // Fall back to `view` where the gradient is degenerate (a flat plateau).
        // A zero normal makes `half` below `normalize(0)` -> NaN, which poisons
        // the accumulation buffer.
        normal = sample_normal(uv, classified, view);
        let half = normalize(view + reflect(-view, normal));
        let F = fresnel(dot(view, half), surface.eta);

        // Russian roulette: the specular lobes run with probability ~F and are
        // weighted back to F, so weak reflections rarely pay for them.
        let p = min(F / SPECULAR_ROULETTE, 1.0);
        specular = select(0.0, F / p, noise.coin < p);
    }

    return sample_outgoing_radiance(uv, noise, normal, view, surface.albedo, specular, surface.Fdr);
}

// Fresnel factor at and above which the specular lobes always run.
const SPECULAR_ROULETTE: f32 = 0.05;

// Outgoing radiance at a real collision from one traced ray to the
// environment: one-sample MIS (balance heuristic) over bounded VNDF sampling of
// the GGX lobe and hierarchical warping of the environment, with a uniform
// share against blocked bright light. The phase is isotropic.
fn traced_scatter(uv: vec3<f32>, direction: vec3<f32>, classified: Classified, material: Material, view: vec3<f32>, pixel: vec2<u32>) -> vec3<f32> {
    let surface = interface_at(uv, direction, material);
    let diffuse = HDRI_SETTINGS.phase * surface.albedo * (1.0 - surface.Fdr) / (4.0 * PI);

    var frame = orthonormal_frame(view);
    var reflects = false;

    if (surface.eta != 1.0) {
        frame = orthonormal_frame(sample_normal(uv, classified, view));
        reflects = dot(frame[2], view) > 0.0;
    }

    let wi = view * frame;
    let alpha = max(HDRI_SETTINGS.roughness * HDRI_SETTINGS.roughness, 1e-3);

    // VNDF takes the specular lobe's share of the energy, by its mirror Fresnel.
    let mirror = select(0.0, fresnel(wi.z, surface.eta), reflects);
    var specular = mirror / max(mirror + 4.0 * PI * brightness(diffuse), EPSILON);

    let strategy = HDRI_SETTINGS.traced_strategy;
    if (strategy == STRATEGY_ENVIRONMENT) { specular = 0.0; }
    if (strategy == STRATEGY_REFLECTION && reflects) { specular = 1.0; }

    let mean = environment_mean();
    let share = select(1.0, HDRI_SETTINGS.traced_uniform_share, mean > 0.0);

    let xi = noise(pixel, 3u);
    var omega = vec3<f32>(0.0);

    if (xi.x < specular) {
        omega = frame * sample_bounded_vndf(wi, alpha, xi.yz);
    } else if (xi.w < share) {
        omega = uniform_sphere(xi.yz);
    } else {
        omega = sample_environment(xi.yz);
    }

    let wo = omega * frame;

    var pdf = (1.0 - specular) * (share / (4.0 * PI) + select(0.0, (1.0 - share) * environment_pdf(omega), mean > 0.0));
    if (reflects) { pdf += specular * bounded_vndf_pdf(wi, wo, alpha); }

    var f = diffuse;

    if (reflects && wo.z > 0.0) {
        let h = normalize(wi + wo);
        let alpha2 = alpha * alpha;
        let F = select(fresnel(dot(wi, h), surface.eta), mirror, MIRROR_FRESNEL);
        f += F * D_GGX(h.z, alpha) * GGX_V1(wi.z, alpha2) * GGX_V1(wo.z, alpha2) * wo.z;
    }

    let radiance = f * environment_radiance(omega);
    if (pdf <= 0.0 || all(radiance <= vec3<f32>(0.0))) { return vec3<f32>(0.0); }

    // To the crop box, through the volume and (as the cascade sees them) lines.
    let step = linear_part(TRANSFORM) * omega;
    let crop = CLASSIFY.crop;
    let exit = intersectAABB(uv, step, max(0.5 + crop.min.xyz, vec3<f32>(0.0)), min(0.5 + crop.max.xyz, vec3<f32>(1.0))).y;
    let lines = CASCADE_OPTS.include_lines != 0u;

    return radiance * ratio_tracking(uv, step, 0.0, exit, lines, pixel, 4u) / pdf;
}

// HdriSettings.traced_strategy; 0 is MIS.
const STRATEGY_ENVIRONMENT: u32 = 1u;
const STRATEGY_REFLECTION: u32 = 2u;

fn average(v: vec3<f32>) -> f32 {
    return (v.x + v.y + v.z) / 3.0;
}

// Matches `to_material`'s reduced scattering.
fn scattering_scale() -> f32 {
    return 1.0 / (1.0 - HDRI_SETTINGS.anisotropy);
}

struct Material {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    extinction: vec3<f32>,
    ior: f32
};

fn to_material(sum: Classified) -> Material {
    let scattering = sum.scattering / (1.0 - HDRI_SETTINGS.anisotropy);

    return Material(sum.absorption, scattering, sum.absorption + scattering, average_ior(sum.properties));
}

// A gradient maps to world by the transpose of the world → uv transform.
fn sample_normal(uv: vec3<f32>, classified: Classified, fallback: vec3<f32>) -> vec3<f32> {
    let normal = -(transpose(linear_part(TRANSFORM)) * sample_gradient(uv, classified));
    if (length(normal) <= 1E-6) { return fallback; }

    return normalize(normal);
}

// Per pixel and frame. `tent` in [-1, 1]³ jitters the probe lookup:
// averaged by accumulation, trilinear interpolation becomes a smooth B-spline.
// `box` in [-½, ½)³ turns a nearest-probe read into trilinear on average,
// `xi` in [0, 1) picks the lobe and `coin` in [0, 1) plays the specular roulette.
struct StepNoise {
    tent: vec3<f32>,
    box: vec3<f32>,
    xi: f32,
    coin: f32,
}

fn sample_outgoing_radiance(
    uv: vec3<f32>,
    noise: StepNoise,
    normal: vec3<f32>,
    view: vec3<f32>,
    albedo: vec3<f32>,
    specular: f32,
    Fdr: f32) -> vec3<f32>
{
    let anisotropy = HDRI_SETTINGS.anisotropy;

    let shading = Shading(
        specular_kernel(normal, view, HDRI_SETTINGS.roughness),
        phase_kernel(view, anisotropy),
        abs(anisotropy) < 1e-3,
        specular > 0.0,
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

    return radiance.specular * specular + HDRI_SETTINGS.phase * radiance.phase * albedo * (1.0 - Fdr);
}

// The kernels the light lobes are integrated against. Isotropic phase is
// precomputed per probe by cascade 0.
struct Shading {
    specular: Kernel,
    phase: Kernel,
    isotropic: bool,
    // Whether the specular sum is weighted in at all.
    reflects: bool,
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

        for (var k = 0u; k < select(0u, lobes, shading.reflects); k += 4u) {
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

fn intersectAABB(origin: vec3<f32>, direction: vec3<f32>, boxMin: vec3<f32>, boxMax: vec3<f32>) -> vec2<f32> {
    let invDir = 1.0 / direction;

    let t1 = (boxMin - origin) * invDir;
    let t2 = (boxMax - origin) * invDir;

    let tMinVec = min(t1, t2);
    let tMaxVec = max(t1, t2);

    let tMin = max(max(tMinVec.x, tMinVec.y), tMinVec.z);
    let tMax = min(min(tMaxVec.x, tMaxVec.y), tMaxVec.z);

    return vec2<f32>(tMin, tMax);
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

fn grid_lobe(grid: ProbeGrid, k: u32) -> Lobe {
    return sg_lobe(probe_grid_sg(RADIANCE, GAUSSIAN, SAMPLER, grid, k));
}
