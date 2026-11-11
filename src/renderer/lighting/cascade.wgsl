use color;
use em/em;
use gaussian;
use math;
use occlusion;
use uniforms;

@group(0) @binding(0) var PHI_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(1) var VMM_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var PHI_IN: texture_3d<f32>;
@group(0) @binding(4) var VMM_IN: texture_3d<f32>;
@group(0) @binding(5) var IRRADIANCE_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(6) var IRRADIANCE_IN: texture_3d<f32>;
@group(0) @binding(8) var<uniform> CASCADE_OPTS: CascadeOpts;
@group(0) @binding(9) var WEIGHTS_OUT: texture_storage_3d<rgba8unorm, write>;

@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var PROPERTIES: texture_3d<f32>;
@group(1) @binding(4) var GRADIENT: texture_3d<f32>;
@group(1) @binding(5) var SAMPLER: sampler;

@group(1) @binding(8) var LINE_EXTINCTION: texture_3d<f32>;
@group(1) @binding(9) var LINE_EXTINCTION_SAMPLER: sampler;
@group(1) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(7) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;
@group(3) @binding(3) var<storage, read> VMM_HDRI: array<vec4<f32>>;
@group(3) @binding(4) var<storage, read> PHI_HDRI: array<vec4<f32>>;
@group(3) @binding(5) var<storage, read> CELLS_HDRI: array<EnvironmentCell>;

// LEVEL SAMPLES  WORKGROUP  SAMPLES_PER_THREAD
//     0      32         32                   1
//     1      32         32                   1
//     2     128         32                   4
//     3     256         64                   4
//     4    1024        256                   4
//     5    4096       1024                   4

const CASCADE_MAX: u32 = 5u;
const WORKGROUP: u32 = 32u;

const SAMPLES: u32 = WORKGROUP * SAMPLES_PER_THREAD;

// Every probe takes the cheapest of:
// - EMPTY: no extinction within EMPTY_MARGIN cells. Never read, so it writes
//   nothing: the renderer reads up to LOOKUP_JITTER (2) + 1 cells from what
//   it shades, and a child sits within one parent cell of its parents.
// - DARK: its parent sees less mean radiance (Σ E / 4π) than IRRADIANCE_CULL of
//   the sky's. Stores zero lobes.
// - COPY: no neighbour has LOCAL_MATTER optical depth across a probe cell, so
//   nothing nearby blocks the parent's light. Stores the parent's lobes.
// - SOLVE: traces and fits.
const EMPTY_MARGIN: f32 = 4.0;
const IRRADIANCE_CULL: f32 = 0.01;
const LOCAL_MATTER: f32 = 1e-3;

const SOLVE: u32 = 0u;
const COPY: u32 = 1u;
const DARK: u32 = 2u;

// A uniform sample covers a solid angle of 4π / SAMPLES ≈ π tan²θ, an
// importance sample that times its density ratio.
const CONE_TAN: f32 = 2.0 / sqrt(f32(SAMPLES));

// How much of each parent lobe the prefiltered samples pick up together.
var<workgroup> PICKUP: array<f32, VMM_MAX>;

var<workgroup> CULL_OCCUPIED: atomic<u32>;
var<workgroup> CULL_RESULT: u32;

const CULL_NEIGHBORS: u32 = 6u;
const NEIGHBOR_OFFSET = array<vec3<f32>, 6>(
    vec3<f32>(-1.0,  0.0,  0.0), vec3<f32>( 1.0,  0.0,  0.0),
    vec3<f32>( 0.0, -1.0,  0.0), vec3<f32>( 0.0,  1.0,  0.0),
    vec3<f32>( 0.0,  0.0, -1.0), vec3<f32>( 0.0,  0.0,  1.0),
);

fn cascade() -> u32 {
    return CASCADE_OPTS.cascade;
}

// Compiled per lobe count: workgroup memory scales with VMM_MAX, and that
// bounds occupancy.
fn vmm_size() -> u32 {
    return VMM_MAX;
}

// Luminance, like the environment's cells. Log compression keeps bright
// sources from claiming every lobe.
fn radiance_weight(radiance: vec3<f32>) -> f32 {
    let luminance = brightness(radiance);
    return select(luminance, log(1.0 + luminance), HDRI_SETTINGS.log_weights != 0u);
}

@compute
@workgroup_size(WORKGROUP)
fn main(
    @builtin(workgroup_id) probe: vec3<u32>,
    @builtin(local_invocation_index) index: u32,
    @builtin(subgroup_id) subgroup: u32,
    @builtin(subgroup_invocation_id) lane: u32,
    @builtin(num_subgroups) subgroups: u32,
) {
    let thread = Thread(index, subgroup, lane, subgroups);
    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << cascade());

    if (is_empty(origin, thread)) { return; }

    let kind = classify(origin, thread);

    if (thread.index < vmm_size()) {
        if (kind == DARK) {
            VMM[thread.index] = vec4<f32>(0.0);
            PHI[thread.index] = vec4<f32>(0.0);
        } else {
            initialize_lobe(probe, thread.index);
        }
    }

    workgroupBarrier();

    if (kind != SOLVE) {
        if (thread.index < vmm_size()) { store_lobe(probe, thread.index); }

        if (cascade() == 0u) {
            store_phase(probe, thread);
        } else if (thread.index == 0u) {
            store_irradiance(probe);
        }
        return;
    }

    let omega = sample_directions(thread);
    let ratio = sample_ratios(thread);
    let radiance = incident_radiance(origin, omega, ratio, thread);
    let weight = radiance_weights(radiance);

    fit_vmm(omega, radiance, weight, ratio, thread);

    if (thread.index < vmm_size()) { store_lobe(probe, thread.index); }

    if (cascade() == 0u) {
        store_phase(probe, thread);
    } else if (thread.index == 0u) {
        store_irradiance(probe);
    }
}

// Eight lookups, each at a mip whose texels span half the margin box, so
// together they see every voxel within EMPTY_MARGIN cells.
fn is_empty(origin: vec3<f32>, thread: Thread) -> bool {
    let dim = vec3<f32>(probe_grid(textureDimensions(VMM_OUT))) * f32(1u << cascade());
    let cell_sample = f32(1u << cascade()) / dim;
    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;
    let mip = ceil(log2((EMPTY_MARGIN + 0.5) * f32(1u << cascade()) * radiance_scale));

    if (thread.index == 0u) { atomicStore(&CULL_OCCUPIED, 0u); }

    workgroupBarrier();

    if (thread.index < 8u) {
        let corner = vec3<f32>(vec3<u32>(thread.index & 1u, (thread.index >> 1u) & 1u, thread.index >> 2u)) * 2.0 - 1.0;
        let sample = (origin + corner * 0.5 * EMPTY_MARGIN * f32(1u << cascade())) / dim;
        if (any(extinction_at(sample, mip, 1.0) > vec3<f32>(0.0))) { atomicOr(&CULL_OCCUPIED, 1u); }
    }

    workgroupBarrier();

    if (thread.index == 0u) { CULL_RESULT = select(0u, 1u, atomicLoad(&CULL_OCCUPIED) == 0u); }

    return workgroupUniformLoad(&CULL_RESULT) != 0u;
}

fn classify(origin: vec3<f32>, thread: Thread) -> u32 {
    let dim = vec3<f32>(probe_grid(textureDimensions(VMM_OUT))) * f32(1u << cascade());
    let origin_sample = origin / dim;
    let cell_sample = f32(1u << cascade()) / dim;

    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;
    let mip = f32(cascade()) + log2(radiance_scale);

    if (thread.index == 0u) { atomicStore(&CULL_OCCUPIED, 0u); }

    workgroupBarrier();

    if (thread.index < CULL_NEIGHBORS) {
        let sample = origin_sample + NEIGHBOR_OFFSET[thread.index] * cell_sample;
        let depth = extinction_at(sample, mip, cell_mm());
        if (any(depth > vec3<f32>(LOCAL_MATTER))) { atomicOr(&CULL_OCCUPIED, 1u); }
    }

    workgroupBarrier();

    if (thread.index == 0u) {
        var kind = select(SOLVE, COPY, atomicLoad(&CULL_OCCUPIED) == 0u);
        if (cascade() < CASCADE_MAX) {
            let irradiance = unpack_rgb(textureSampleLevel(IRRADIANCE_IN, SAMPLER, origin_sample, 0.0));
            // Environments are normalized to mean luminance 1 at unit strength.
            if (brightness(irradiance) < IRRADIANCE_CULL * HDRI_SETTINGS.strength) { kind = DARK; }
        }
        CULL_RESULT = kind;
    }

    return workgroupUniformLoad(&CULL_RESULT);
}

// This level's probe spacing in mm.
fn cell_mm() -> f32 {
    let dim = vec3<f32>(probe_grid(textureDimensions(VMM_OUT))) * f32(1u << cascade());
    return f32(1u << cascade()) / length(dim * (linear_part(TRANSFORM) * vec3<f32>(1.0, 0.0, 0.0)));
}

fn initialize_lobe(probe: vec3<u32>, k: u32) {
    if (cascade() == CASCADE_MAX) {
        let vmm = VMM_HDRI[vmm_size() + k];
        let phi = PHI_HDRI[vmm_size() + k];

        VMM[k] = vec4<f32>(rotate_hdri(vmm.xyz) * length(vmm.xyz), vmm.w);
        PHI[k] = vec4<f32>(phi.rgb * HDRI_SETTINGS.strength, phi.w);
    } else {
        let uv = parent_uv(textureDimensions(VMM_IN), probe, k);

        VMM[k] = textureSampleLevel(VMM_IN, SAMPLER, uv, 0.0);
        PHI[k] = textureSampleLevel(PHI_IN, SAMPLER, uv, 0.0);
    }

    VMM_PRIOR[k] = VMM[k];
}

// Both sample sets live in the environment's space and turn with it, so the
// same directions relative to the HDRI are always traced. Importance sampling
// traces the environment's cut, SAMPLES cells at [SAMPLES, 2 SAMPLES).
fn sample_direction(i: u32) -> vec3<f32> {
    if (HDRI_SETTINGS.importance == 0u) { return rotate_hdri(hammersley_direction(i, SAMPLES)); }

    return rotate_hdri(CELLS_HDRI[SAMPLES + i].direction);
}

fn sample_ratio(i: u32) -> f32 {
    if (HDRI_SETTINGS.importance == 0u) { return 1.0; }

    return CELLS_HDRI[SAMPLES + i].ratio;
}

// Lobes live in world space, where solid angles are physical. The HDRI fit and
// its samples are baked unrotated.
fn rotate_hdri(direction: vec3<f32>) -> vec3<f32> {
    return rotation_z(-HDRI_SETTINGS.rotation * 2.0 * PI) * normalize(direction);
}

fn store_lobe(probe: vec3<u32>, k: u32) {
    let texel = probe + probe_grid(textureDimensions(VMM_OUT)) * lobe_tile(k);

    // Energy and the solid angle it covers, so interpolation keeps energy.
    textureStore(VMM_OUT, texel, VMM[k]);
    textureStore(PHI_OUT, texel, PHI[k]);
}

// Cascade 0's irradiance mip is read by the renderer, not by culling: it
// holds the isotropic phase radiance Σ E / 4π, unpacked.
// Cascade 0 has one lobe per thread.
fn store_phase(probe: vec3<u32>, thread: Thread) {
    var phase = vec3<f32>(0.0);

    if (thread.index < vmm_size()) {
        // ∫ L dω / 4π of the lobe.
        phase = PHI[thread.index].rgb / (4.0 * PI);
    }

    let rounds = subgroup_rounds(thread);

    for (var round=0u; round<rounds; round++) {
        scatter_partial(0u, round, thread, vec4<f32>(phase, 0.0));
        workgroupBarrier();
    }

    if (thread.index == 0u) { textureStore(IRRADIANCE_OUT, probe, vec4<f32>(gather_partials(0u, thread).rgb, 1.0)); }

    if (thread.index < (vmm_size() + 3u) / 4u) { store_weights(probe, thread.index); }
}

// The volume tracer picks one lobe per step by these: each lobe's energy over
// the probe's brightest, rounded up to 8 bits so no lit lobe gets p = 0.
// Texel j holds lobes 4j..4j+3.
fn store_weights(probe: vec3<u32>, j: u32) {
    var brightest = EPSILON;
    for (var k=0u; k<vmm_size(); k++) { brightest = max(brightest, brightness(PHI[k].rgb)); }

    var weights = vec4<f32>(0.0);

    for (var i=0u; i<4u; i++) {
        let k = 4u * j + i;
        if (k < vmm_size()) { weights[i] = min(ceil(255.0 * brightness(PHI[k].rgb) / brightest), 255.0) / 255.0; }
    }

    let grid = probe_grid(textureDimensions(VMM_OUT));
    let tile = vec3<u32>(j & 1u, (j >> 1u) & 1u, (j >> 2u) & 1u);

    textureStore(WEIGHTS_OUT, probe + grid * tile, weights);
}

// The probe's mean radiance over the sphere, Σ E / 4π, read by culling.
fn store_irradiance(probe: vec3<u32>) {
    var energy = vec3<f32>(0.0);

    for (var k=0u; k<vmm_size(); k++) {
        energy += PHI[k].rgb;
    }

    textureStore(IRRADIANCE_OUT, probe, pack_rgb(energy / (4.0 * PI)));
}

fn incident_radiance(origin: vec3<f32>, omega: Directions, ratio: Weights, thread: Thread) -> Radiances {
    var radiance = Radiances();

    if (HDRI_SETTINGS.prefilter != 0u) {
        radiance = prefiltered_radiance(omega, ratio, thread);
    } else {
        radiance = parent_radiance(omega);
    }

    return hadamard(radiance, trace_samples(origin, omega, ratio));
}

// The parent's lobes as a soft partition of the sphere: each direction takes
// its lobes' mean radiances, weighted by their vMF densities.
fn parent_radiance(omega: Directions) -> Radiances {
    var radiance = Radiances();
    var expectation_sum = Weights();

    for (var k=0u; k<vmm_size(); k++) {
        if (PHI[k].w < EPSILON) { continue; }

        let expectation = vmf(VMM[k], omega);
        expectation_sum += expectation;

        radiance += outer(mean_radiance(PHI[k]).rgb, expectation);
    }

    return scale(radiance, 1.0 / expectation_sum);
}

// Each parent lobe's energy (Σ γ L Ω) spread over the samples by its overlap
// with each sample's footprint Ω = 4π·ratio / SAMPLES, a normalized vMF of
// κs ≈ 2π / Ω: vMF ⊛ vMF ≈ vMF with 1/κ' = 1/κ + 1/κs. Footprints of unequal
// cells overlap, so each lobe is normalized by what all samples pick up of it:
// every lobe delivers exactly its energy, and no small light falls between samples.
fn prefiltered_radiance(omega: Directions, ratio: Weights, thread: Thread) -> Radiances {
    let kappa_s = f32(SAMPLES) / (2.0 * ratio);
    let solid_angle = ratio * (4.0 * PI / f32(SAMPLES));
    let rounds = subgroup_rounds(thread);

    for (var round=0u; round<rounds; round++) {
        for (var k=0u; k<vmm_size(); k++) {
            scatter_partial(k, round, thread, vec4<f32>(total(solid_angle * footprint(k, omega, kappa_s))));
        }

        workgroupBarrier();
    }

    if (thread.index < vmm_size()) { PICKUP[thread.index] = gather_partials(thread.index, thread).x; }

    workgroupBarrier();

    var radiance = Radiances();

    for (var k=0u; k<vmm_size(); k++) {
        radiance += outer(PHI[k].rgb / max(PICKUP[k], EPSILON), footprint(k, omega, kappa_s));
    }

    return radiance;
}

// Lobe k's normalized vMF, widened by the sample footprint κs, at omega.
fn footprint(k: u32, omega: Directions, kappa_s: Weights) -> Weights {
    if (PHI[k].w < EPSILON) { return Weights(); }

    let lobe = sg_lobe(SG(VMM[k], PHI[k].rgb));
    let kappa = lobe.kappa * kappa_s / (lobe.kappa + kappa_s);
    let norm = INV_TWO_PI * kappa / (1.0 - exp(-2.0 * kappa));

    return norm * exp(kappa * (project(lobe.axis, omega) - 1.0));
}

// Interval [2^(c+1) − 2, 2^(c+2) − 2) in probe cells: contiguous from 0, each
// twice its level's probe spacing.
fn trace(origin: vec3<f32>, direction: vec3<f32>, ratio: f32) -> vec3<f32> {
    let c = cascade();
    return transmission(origin, direction, CONE_TAN * sqrt(ratio), f32((2u << c) - 2u), f32((4u << c) - 2u));
}

// Marches the world `direction` in probe cells: t, the interval and the cone are
// in cells.
fn transmission(origin: vec3<f32>, direction: vec3<f32>, cone_tan: f32, t0: f32, t1: f32) -> vec3<f32> {
    var transmission = vec3<f32>(1.0);

    let dim = vec3<f32>(probe_grid(textureDimensions(VMM_OUT))) * f32(1u << cascade());
    let origin_sample = origin / dim;

    let cells = dim * (linear_part(TRANSFORM) * direction);
    let scale = length(cells); // cells/mm along `direction`
    let direction_sample = cells / (scale * dim);

    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;
    let voxel = 1.0 / radiance_scale;

    var t = t0;

    while (t < t1) {
        let diameter = max(2.0 * cone_tan * t, voxel);
        let step = min(diameter, t1 - t);
        let sample = origin_sample + direction_sample * (t + 0.5 * step);

        if (any(abs(sample - 0.5) > vec3<f32>(0.5))) { break; }

        let depth = extinction_at(sample, log2(diameter / voxel), step / scale);

        transmission *= exp(-depth);

        if (all(transmission < vec3<f32>(1e-3))) { break; }

        t += step;
    }

    return transmission;
}

// Optical depth over a `segment` of mm: the volume's `EXTINCTION` and/or the
// line deposit's `LINE_EXTINCTION`, per this radiance buffer's `cascade_opts`.
fn extinction_at(uv: vec3<f32>, mip: f32, segment: f32) -> vec3<f32> {
    var e = vec3<f32>(0.0);
    if (CASCADE_OPTS.include_volume != 0u) {
        e += segment * unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, uv, mip));
    }
    if (CASCADE_OPTS.include_lines != 0u) {
        e += vec3<f32>(segment * textureSampleLevel(LINE_EXTINCTION, LINE_EXTINCTION_SAMPLER, uv, mip).x);
    }
    return e;
}

fn parent_uv(dim: vec3<u32>, probe: vec3<u32>, k: u32) -> vec3<f32> {
    let half = vec3<f32>(0.5);
    let parent_grid = vec3<f32>(probe_grid(dim));
    let parent_probe = clamp(half * (vec3<f32>(probe) + half), half, parent_grid - half);
    return (parent_probe + parent_grid * vec3<f32>(lobe_tile(k))) / vec3<f32>(dim);
}

fn probe_grid(dim: vec3<u32>) -> vec3<u32> {
    return max(dim / vec3<u32>(4, 4, 2), vec3<u32>(1));
}

fn lobe_tile(k: u32) -> vec3<u32> {
    return vec3<u32>(k & 3, (k >> 2) & 3, (k >> 4) & 1);
}

fn mean_radiance(v: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(v.rgb / max(v.w, EPSILON), 1.0);
}
