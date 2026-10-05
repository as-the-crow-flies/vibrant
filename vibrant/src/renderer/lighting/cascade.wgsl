struct CascadeOpts {
    include_volume: u32,
    include_lines: u32,
}

@group(0) @binding(0) var PHI_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(1) var VMM_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var PHI_IN: texture_3d<f32>;
@group(0) @binding(4) var VMM_IN: texture_3d<f32>;
@group(0) @binding(5) var IRRADIANCE_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(6) var IRRADIANCE_IN: texture_3d<f32>;
@group(0) @binding(8) var<storage, read> PHI_HDRI_IN: array<vec4<f32>>;
@group(0) @binding(9) var<storage, read> VMM_HDRI_IN: array<vec4<f32>>;
@group(0) @binding(10) var<uniform> CASCADE_OPTS: CascadeOpts;

// Diagnostic Histogram
@group(0) @binding(7) var<storage, read_write> EM_ITERATIONS: array<atomic<u32>>;

@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var PROPERTIES: texture_3d<f32>;
@group(1) @binding(4) var GRADIENT: texture_3d<f32>;
@group(1) @binding(5) var SAMPLER: sampler;

@group(1) @binding(8) var LINE_EXTINCTION: texture_3d<f32>;
@group(1) @binding(9) var LINE_EXTINCTION_SAMPLER: sampler;
@group(1) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(7) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;

// LEVEL SAMPLES  WORKGROUP  SUBGROUPS  SAMPLES_PER_THREAD
//     0      32         32          1                   1
//     1      32         32          1                   1
//     2     128         32          1                   4
//     3     256         64          2                   4
//     4    1024        256          8                   4
//     5    4096       1024         32                   4

const CASCADE_MAX: u32 = 5u;
const CASCADE: u32 = #CASCADE;
const WORKGROUP: u32 = #WORKGROUP;
const SUBGROUPS: u32 = WORKGROUP / 32u;
const SAMPLES: u32 = WORKGROUP * SAMPLES_PER_THREAD;

const IRRADIANCE_CULL: f32 = 0.05;

// Each sample covers a solid angle of 4π / SAMPLES ≈ π tan²θ.
const CONE_TAN: f32 = 2.0 / sqrt(f32(SAMPLES));

var<workgroup> VMM: array<vec4<f32>, VMM_SIZE>;
var<workgroup> VMM_PRIOR: array<vec4<f32>, VMM_SIZE>;

var<workgroup> PHI: array<vec4<f32>, VMM_SIZE>;

var<workgroup> SUM: array<vec4<f32>, VMM_SIZE * SUBGROUPS>;

var<workgroup> VMM_DELTA: atomic<u32>;
var<workgroup> VMM_CONVERGED: u32;

var<workgroup> CULL_OCCUPIED: atomic<u32>;
var<workgroup> CULL_RESULT: u32;

const CULL_NEIGHBORS: u32 = 6u;
const NEIGHBOR_OFFSET = array<vec3<f32>, 6>(
    vec3<f32>(-1.0,  0.0,  0.0), vec3<f32>( 1.0,  0.0,  0.0),
    vec3<f32>( 0.0, -1.0,  0.0), vec3<f32>( 0.0,  1.0,  0.0),
    vec3<f32>( 0.0,  0.0, -1.0), vec3<f32>( 0.0,  0.0,  1.0),
);

@compute
@workgroup_size(WORKGROUP)
fn main(
    @builtin(workgroup_id) probe: vec3<u32>,
    @builtin(local_invocation_index) index: u32,
) {
    if (index < VMM_SIZE) { initialize(probe, index); }

    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    workgroupBarrier();

    if (cull(origin, index)) {
        if (index < VMM_SIZE) { store(probe, index); }
        if (index == 0u) { debug_iteration_count(0); }
        return;
    }

    let omega = sample_directions(index);
    let radiance = get_incident_radiance(origin, omega);
    let weight = sample_weights(radiance);

    expectation_maximization(omega, radiance, weight, index);

    if (index < VMM_SIZE) { store(probe, index); }
    if (index == 0u) { store_irradiance(probe); }
}

fn cull(origin: vec3<f32>, index: u32) -> bool {
    let dim = vec3<f32>(grid(textureDimensions(VMM_OUT))) * f32(1u << CASCADE);
    let origin_sample = origin / dim;
    let cell_sample = f32(1u << CASCADE) / dim;

    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;
    let mip = f32(CASCADE) + log2(radiance_scale);

    if (index == 0u) { atomicStore(&CULL_OCCUPIED, 0u); }

    workgroupBarrier();

    if (index < CULL_NEIGHBORS) {
        let sample = origin_sample + NEIGHBOR_OFFSET[index] * cell_sample;
        let extinction = sample_extinction(sample, mip);
        if (any(extinction > vec3<f32>(EPSILON))) { atomicOr(&CULL_OCCUPIED, 1u); }
    }

    workgroupBarrier();

    var result = atomicLoad(&CULL_OCCUPIED) == 0u;
    if (!result && CASCADE < CASCADE_MAX) {
        let irradiance = unpack_rgb(textureSampleLevel(IRRADIANCE_IN, SAMPLER, origin_sample, 0.0));
        result = brightness(irradiance) < IRRADIANCE_CULL;
    }

    if (index == 0u) { CULL_RESULT = select(0u, 1u, result); }

    return workgroupUniformLoad(&CULL_RESULT) != 0u;
}

fn initialize(probe: vec3<u32>, index: u32) {
    if (CASCADE == CASCADE_MAX) {
        VMM[index] = VMM_HDRI_IN[index];
        PHI[index] = PHI_HDRI_IN[index];
    } else {
        let uv = parent_uv(textureDimensions(VMM_IN), probe, index);

        VMM[index] = textureSampleLevel(VMM_IN, SAMPLER, uv, 0.0);
        PHI[index] = textureSampleLevel(PHI_IN, SAMPLER, uv, 0.0);
    }

    VMM_PRIOR[index] = VMM[index];
}

fn expectation_maximization(omega: Directions, radiance: Radiances, weight: Weights, index: u32) {
    let subgroup = index >> 5u;
    let subgroup_index = index & 31;

    var expectation = array<Weights, VMM_SIZE>();
    var expectation_sum_inv = Weights();

    var iteration = 1u;

    for (; iteration<=EM_ITERATIONS_MAX; iteration++) {
        if (index == 0u) { atomicStore(&VMM_DELTA, 0u); }

        // Expectation
        var expectation_sum = Weights();

        for (var k=0u; k<VMM_SIZE; k++) {
            expectation[k] = vmf(VMM[k], omega);
            expectation_sum += expectation[k];
        }

        expectation_sum_inv = 1.0 / max(expectation_sum, Weights(EPSILON));

        workgroupBarrier();

        // Maximization
        for (var k=0u; k<VMM_SIZE; k++) {
            let gamma_weight = expectation[k] * expectation_sum_inv * weight;
            let vmm = vec4<f32>(omega * gamma_weight, total(gamma_weight));
            scatter_partial(k, subgroup, subgroup_index, vmm);
        }

        workgroupBarrier();

        gather_partial(subgroup, subgroup_index);

        workgroupBarrier();

        if (converged(iteration, index)) { break; }
    }

    if (index == 0u) { debug_iteration_count(iteration); }

    // Maximize Phi
    for (var k=0u; k<VMM_SIZE; k++) {
        let gamma = expectation[k] * expectation_sum_inv;
        let phi = vec4<f32>(radiance * gamma, total(gamma));
        scatter_partial(k, subgroup, subgroup_index, phi);
    }

    workgroupBarrier();

    gather_partial_phi(subgroup, subgroup_index);

    workgroupBarrier();
}

fn converged(iteration: u32, index: u32) -> bool {
    if (index == 0u) {
        let done = iteration >= EM_ITERATIONS_MIN && bitcast<f32>(atomicLoad(&VMM_DELTA)) < EM_CONVERGENCE;
        VMM_CONVERGED = select(0u, 1u, done);
    }

    return workgroupUniformLoad(&VMM_CONVERGED) != 0u;
}

fn store(probe: vec3<u32>, index: u32) {
    let texel = probe + grid(textureDimensions(VMM_OUT)) * lobe(index);

    let vmm = VMM[index];
    let phi = max_norm(PHI[index]);

    textureStore(VMM_OUT, texel, vmm);
    textureStore(PHI_OUT, texel, phi);
}

fn store_irradiance(probe: vec3<u32>) {
    var irradiance = vec3<f32>(0.0);

    for (var k=0u; k<VMM_SIZE; k++) {
        irradiance += max_norm(PHI[k]).rgb;
    }

    textureStore(IRRADIANCE_OUT, probe, pack_rgb(irradiance));
}

fn get_incident_radiance(origin: vec3<f32>, omega: Directions) -> Radiances {
    var radiance = Radiances();
    var expectation_sum = Weights();

    for (var k=0u; k<VMM_SIZE; k++) {
        if (PHI[k].w < EPSILON) { continue; }

        let expectation = vmf(VMM[k], omega);
        expectation_sum += expectation;

        radiance += outer(PHI[k].rgb / PHI[k].w, expectation);
    }

    radiance = scale(radiance, 1.0 / expectation_sum);

    return hadamard(radiance, trace_samples(origin, omega));
}

fn trace(origin: vec3<f32>, direction: vec3<f32>) -> vec3<f32> {
    return transmission(origin, direction, INTERVAL[CASCADE], INTERVAL[CASCADE + 1]);
}

fn transmission(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> vec3<f32> {
    var transmission = vec3<f32>(1.0);

    let dim = vec3<f32>(grid(textureDimensions(VMM_OUT))) * f32(1u << CASCADE);
    let origin_sample = origin / dim;
    let direction_sample = direction / dim;

    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;

    let scale = length(TRANSFORM[0].xyz) * dim.x; // voxels/mm
    let voxel = 1.0 / radiance_scale;

    var t = t0;

    while (t < t1) {
        let diameter = max(2.0 * CONE_TAN * t, voxel);
        let step = min(diameter, t1 - t);
        let sample = origin_sample + direction_sample * (t + 0.5 * step);

        if (any(abs(sample - 0.5) > vec3<f32>(0.5))) { break; }

        let extinction = sample_extinction(sample, log2(diameter / voxel));

        transmission *= exp(-extinction * step / scale);

        if (all(transmission < vec3<f32>(1e-3))) { break; }

        t += step;
    }

    return transmission;
}

// Occlusion extinction for the cascade: the volume's `EXTINCTION` and/or the
// line deposit's `LINE_EXTINCTION`, per this radiance buffer's `cascade_opts`.
fn sample_extinction(sample: vec3<f32>, mip: f32) -> vec3<f32> {
    var e = vec3<f32>(0.0);
    if (CASCADE_OPTS.include_volume != 0u) {
        e += unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, sample, mip));
    }
    if (CASCADE_OPTS.include_lines != 0u) {
        e += vec3<f32>(textureSampleLevel(LINE_EXTINCTION, LINE_EXTINCTION_SAMPLER, sample, mip).x);
    }
    return e;
}

fn scatter_partial(k: u32, subgroup: u32, subgroup_index: u32, item: vec4<f32>) {
    let sum = subgroupAdd(item);

    if (subgroup_index == 0u) {
        let i = SUBGROUPS * k + subgroup;
        SUM[i] = sum;
    }
}

fn gather_partial(subgroup: u32, subgroup_index: u32) {
    let m = subgroup_index / SUBGROUPS;
    let s = subgroup_index % SUBGROUPS;
    let k = subgroup + m * SUBGROUPS;
    let i = SUBGROUPS * k + s;

    var vmm = SUM[i];

    for (var offset = 1u; offset < SUBGROUPS; offset *= 2u) {
        vmm += subgroupShuffleXor(vmm, offset);
    }

    if (s == 0u && k < VMM_SIZE) {
        let vmm_prior = vmm + VMM_PRIOR[k];

        let change = length(vmm_prior - VMM[k]);
        let scale = length(vmm_prior) + EPSILON;
        atomicMax(&VMM_DELTA, bitcast<u32>(change / scale));

        VMM[k] = vmm_prior;
    }
}

fn gather_partial_phi(subgroup: u32, subgroup_index: u32) {
    let m = subgroup_index / SUBGROUPS;
    let s = subgroup_index % SUBGROUPS;
    let k = subgroup + m * SUBGROUPS;
    let i = SUBGROUPS * k + s;

    var phi = SUM[i];

    for (var offset = 1u; offset < SUBGROUPS; offset *= 2u) {
        phi += subgroupShuffleXor(phi, offset);
    }

    if (s == 0u && k < VMM_SIZE) {
        PHI[k] = phi;
    }
}

fn parent_uv(dim: vec3<u32>, probe: vec3<u32>, index: u32) -> vec3<f32> {
    let half = vec3<f32>(0.5);
    let parent_grid = vec3<f32>(grid(dim));
    let parent_probe = clamp(half * (vec3<f32>(probe) + half), half, parent_grid - half);
    return (parent_probe + parent_grid * vec3<f32>(lobe(index))) / vec3<f32>(dim);
}

fn debug_iteration_count(iterations: u32) {
    atomicAdd(&EM_ITERATIONS[CASCADE * (EM_ITERATIONS_MAX + 1u) + iterations], 1u);
}
