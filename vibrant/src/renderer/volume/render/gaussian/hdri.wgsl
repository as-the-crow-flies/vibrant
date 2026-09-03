@group(0) @binding(0) var<storage, read_write> PHI_HDRI_OUT: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> VMM_HDRI_OUT: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> EM_ITERATIONS: array<atomic<u32>>;

@group(1) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(7) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;

const SUBGROUPS: u32 = 32u;
const SAMPLES: u32 = 4096u;

const EM_ITERATIONS_HDRI_ROW: u32 = 6u; // == GaussianRadianceBuffer::LEVELS

var<workgroup> VMM: array<vec4<f32>, VMM_SIZE>;
var<workgroup> VMM_PRIOR: array<vec4<f32>, VMM_SIZE>;

var<workgroup> PHI: array<vec4<f32>, VMM_SIZE>;

var<workgroup> SUM: array<vec4<f32>, VMM_SIZE * SUBGROUPS>;

var<workgroup> VMM_DELTA: atomic<u32>;
var<workgroup> VMM_CONVERGED: u32;

@compute
@workgroup_size(1024)
fn main(@builtin(local_invocation_index) index: u32) {
    if (index < VMM_SIZE) { initialize(index); }

    workgroupBarrier();

    let omega = mat4x3<f32>(
        get_direction(4*index+0, SAMPLES),
        get_direction(4*index+1, SAMPLES),
        get_direction(4*index+2, SAMPLES),
        get_direction(4*index+3, SAMPLES),
    );

    let radiance = mat4x3<f32>(
        hdri(omega[0]),
        hdri(omega[1]),
        hdri(omega[2]),
        hdri(omega[3]),
    );

    let weight = vec4<f32>(
        radiance_weight(radiance[0]),
        radiance_weight(radiance[1]),
        radiance_weight(radiance[2]),
        radiance_weight(radiance[3]),
    );

    expectation_maximization(omega, radiance, weight, index);

    if (index < VMM_SIZE) { store(index); }
}

fn initialize(index: u32) {
    VMM[index] = vec4<f32>(get_direction(index, VMM_SIZE), 1.0) / f32(VMM_SIZE);
}

fn expectation_maximization(omega: mat4x3<f32>, radiance: mat4x3<f32>, weight: vec4<f32>, index: u32) {
    let subgroup = index >> 5u;
    let subgroup_index = index & 31;

    var expectation = array<vec4<f32>, VMM_SIZE>();
    var expectation_sum_inv = vec4<f32>(0.0);

    var iteration = 1u;

    for (; iteration<=EM_ITERATIONS_MAX; iteration++) {
        if (index == 0u) { atomicStore(&VMM_DELTA, 0u); }

        // Expectation
        var expectation_sum = vec4<f32>(0.0);

        for (var k=0u; k<VMM_SIZE; k++) {
            expectation[k] = vmf(VMM[k], omega);
            expectation_sum += expectation[k];
        }

        expectation_sum_inv = 1.0 / max(expectation_sum, vec4<f32>(EPSILON));

        workgroupBarrier();

        // Maximization
        for (var k=0u; k<VMM_SIZE; k++) {
            let gamma_weight = expectation[k] * expectation_sum_inv * weight;
            let vmm = vec4<f32>(omega * gamma_weight, sum(gamma_weight));
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
        let phi = vec4<f32>(radiance * gamma, sum(gamma));
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

fn store(index: u32) {
    let vmm = VMM[index];

    let rotated = rotate_hdri(vmm.xyz) * length(vmm.xyz);

    VMM_HDRI_OUT[index] = vec4<f32>(rotated, vmm.w);
    PHI_HDRI_OUT[index] = PHI[index];
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

fn rotate_hdri(direction: vec3<f32>) -> vec3<f32> {
    let world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let world_rotated = rotation_z(-HDRI_SETTINGS.rotation * 2.0 * PI) * world;
    let local = normalize((TRANSFORM * vec4<f32>(world_rotated, 0.0)).xyz);

    return local;
}

fn hdri(direction: vec3<f32>) -> vec3<f32> {
    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, 0.0);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, 0.0).rgb;

    return radiance;
}

fn debug_iteration_count(iterations: u32) {
    atomicAdd(&EM_ITERATIONS[EM_ITERATIONS_HDRI_ROW * (EM_ITERATIONS_MAX + 1u) + iterations], 1u);
}
