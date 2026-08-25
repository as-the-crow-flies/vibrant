@group(0) @binding(0) var PHI_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(1) var VMM_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var PHI_IN: texture_3d<f32>;
@group(0) @binding(4) var VMM_IN: texture_3d<f32>;
@group(0) @binding(5) var IRRADIANCE_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(6) var IRRADIANCE_IN: texture_3d<f32>;

@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var PROPERTIES: texture_3d<f32>;
@group(1) @binding(4) var GRADIENT: texture_3d<f32>;
@group(1) @binding(5) var SAMPLER: sampler;
@group(1) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(7) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;

// LEVEL SAMPLES  WORKGROUP  SUBGROUPS
//     0     128         32          1
//     1     128         32          1
//     2     128         32          1
//     3     256         64          2
//     4    1024        256          8
//     5    4096       1024         32

const CASCADE_MAX: u32 = 5u;
const CASCADE: u32 = #CASCADE;
const WORKGROUP: u32 = #WORKGROUP;
const SUBGROUPS: u32 = #SUBGROUPS;
const SAMPLES: u32 = #SAMPLES;

const EM_ITERATIONS_MAX: u32 = 10u;
const EM_ITERATIONS_MIN: u32 = 2u;
const EM_CONVERGENCE: f32 = 0.02;

const IRRADIANCE_CULL: f32 = 0.1;

var<workgroup> VMM: array<vec4<f32>, VMM_SIZE>;
var<workgroup> PHI: array<vec4<f32>, VMM_SIZE>;

var<workgroup> VMM_PRIOR: array<vec4<f32>, VMM_SIZE>;
var<workgroup> PHI_PRIOR: array<vec4<f32>, VMM_SIZE>;

var<workgroup> VMM_PARTIAL: array<vec4<f32>, VMM_SIZE * SUBGROUPS>;
var<workgroup> PHI_PARTIAL: array<vec4<f32>, VMM_SIZE * SUBGROUPS>;

var<workgroup> VMM_DELTA: atomic<u32>;
var<workgroup> VMM_CONVERGED: u32;

var<workgroup> CULL_OCCUPIED: atomic<u32>;
var<workgroup> CULL_RESULT: u32;

const CULL_NEIGHBORS: u32 = 27u;
const NEIGHBOR_OFFSET = array<vec3<f32>, 27>(
    vec3<f32>( 0.0,  0.0,  0.0),
    vec3<f32>(-1.0, -1.0, -1.0), vec3<f32>( 0.0, -1.0, -1.0), vec3<f32>( 1.0, -1.0, -1.0),
    vec3<f32>(-1.0,  0.0, -1.0), vec3<f32>( 0.0,  0.0, -1.0), vec3<f32>( 1.0,  0.0, -1.0),
    vec3<f32>(-1.0,  1.0, -1.0), vec3<f32>( 0.0,  1.0, -1.0), vec3<f32>( 1.0,  1.0, -1.0),
    vec3<f32>(-1.0, -1.0,  0.0), vec3<f32>( 0.0, -1.0,  0.0), vec3<f32>( 1.0, -1.0,  0.0),
    vec3<f32>(-1.0,  0.0,  0.0),                              vec3<f32>( 1.0,  0.0,  0.0),
    vec3<f32>(-1.0,  1.0,  0.0), vec3<f32>( 0.0,  1.0,  0.0), vec3<f32>( 1.0,  1.0,  0.0),
    vec3<f32>(-1.0, -1.0,  1.0), vec3<f32>( 0.0, -1.0,  1.0), vec3<f32>( 1.0, -1.0,  1.0),
    vec3<f32>(-1.0,  0.0,  1.0), vec3<f32>( 0.0,  0.0,  1.0), vec3<f32>( 1.0,  0.0,  1.0),
    vec3<f32>(-1.0,  1.0,  1.0), vec3<f32>( 0.0,  1.0,  1.0), vec3<f32>( 1.0,  1.0,  1.0),
);

@compute
@workgroup_size(WORKGROUP)
fn main(
    @builtin(workgroup_id) probe: vec3<u32>,
    @builtin(local_invocation_index) index: u32,
) {
    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    if (index < VMM_SIZE) { initialize(probe, index); }

    workgroupBarrier();

    if (cull(origin, index)) {
        if (index < VMM_SIZE) { store(probe, index); }
        if (index == 0u) { store_irradiance(probe); }
        return;
    }

    let omega = mat4x3<f32>(
        get_direction(4*index+0, SAMPLES),
        get_direction(4*index+1, SAMPLES),
        get_direction(4*index+2, SAMPLES),
        get_direction(4*index+3, SAMPLES),
    );

    var radiance = get_incident_radiance(origin, omega);

    let weight = vec4<f32>(
        brightness(radiance[0]),
        brightness(radiance[1]),
        brightness(radiance[2]),
        brightness(radiance[3]),
    );

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
        let extinction = unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, sample, mip));
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
        VMM[index] = vec4<f32>(get_direction(index, VMM_SIZE), 1.0) / f32(VMM_SIZE);
    } else {
        let uv = parent_uv(textureDimensions(VMM_IN), probe, index);

        VMM[index] = textureSampleLevel(VMM_IN, SAMPLER, uv, 0.0);
        PHI[index] = textureSampleLevel(PHI_IN, SAMPLER, uv, 0.0);

        VMM_PRIOR[index] = VMM[index];
        PHI_PRIOR[index] = PHI[index];
    }
}

fn expectation_maximization(omega: mat4x3<f32>, radiance: mat4x3<f32>, weight: vec4<f32>, index: u32) {
    let subgroup = index >> 5u;
    let subgroup_index = index & 31;

    var expectation = array<vec4<f32>, VMM_SIZE>();

    for (var i=0u; i<EM_ITERATIONS_MAX; i++) {
        if (index == 0u) { atomicStore(&VMM_DELTA, 0u); }

        // Expectation
        var expectation_sum = vec4<f32>(0.0);

        for (var k=0u; k<VMM_SIZE; k++) {
            expectation[k] = vmf(VMM[k], omega);
            expectation_sum += expectation[k];
        }

        let expectation_sum_inv = 1.0 / max(expectation_sum, vec4<f32>(EPSILON));

        workgroupBarrier();

        // Maximization
        for (var k=0u; k<VMM_SIZE; k++) {
            let gamma = expectation[k] * expectation_sum_inv;
            let gamma_weight = gamma * weight;

            let vmm = vec4<f32>(omega * gamma_weight, sum(gamma_weight));
            let phi = vec4<f32>(radiance * gamma, sum(gamma));

            scatter_partial(k, subgroup, subgroup_index, vmm, phi);
        }

        workgroupBarrier();

        gather_partial(subgroup, subgroup_index);

        workgroupBarrier();

        if (converged(i, index)) {
            break;
        }
    }
}

fn converged(i: u32, index: u32) -> bool {
    if (index == 0u) {
        let done = i + 1u >= EM_ITERATIONS_MIN && bitcast<f32>(atomicLoad(&VMM_DELTA)) < EM_CONVERGENCE;
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

fn get_incident_radiance(origin: vec3<f32>, omega: mat4x3<f32>) -> mat4x3<f32> {
    var radiance = mat4x3<f32>();

    if (CASCADE == CASCADE_MAX) {
        radiance = get_incident_radiance_hdri(omega);
    } else {
        radiance = get_incident_radiance_parent(omega);
    }

    return mat4x3<f32>(
        radiance[0] * transmission(origin, omega[0], INTERVAL[CASCADE], INTERVAL[CASCADE + 1]),
        radiance[1] * transmission(origin, omega[1], INTERVAL[CASCADE], INTERVAL[CASCADE + 1]),
        radiance[2] * transmission(origin, omega[2], INTERVAL[CASCADE], INTERVAL[CASCADE + 1]),
        radiance[3] * transmission(origin, omega[3], INTERVAL[CASCADE], INTERVAL[CASCADE + 1]),
    );
}

fn get_incident_radiance_hdri(omega: mat4x3<f32>) -> mat4x3<f32> {
    return mat4x3<f32>(
        hdri(omega[0], SAMPLES),
        hdri(omega[1], SAMPLES),
        hdri(omega[2], SAMPLES),
        hdri(omega[3], SAMPLES),
    );
}

fn get_incident_radiance_parent(omega: mat4x3<f32>) -> mat4x3<f32> {
    var radiance = mat4x3<f32>();
    var expectation_sum = vec4<f32>(0.0);

    for (var k=0u; k<VMM_SIZE; k++) {
        let expectation = vmf(VMM[k], omega);
            expectation_sum += expectation;

        let phi = PHI[k];
        let phi_norm = phi.rgb / phi.w;

        radiance += mat4x3<f32>(
            expectation[0] * phi_norm,
            expectation[1] * phi_norm,
            expectation[2] * phi_norm,
            expectation[3] * phi_norm,
        );
    }

    let expectation_sum_inv = 1.0 / expectation_sum;
    return mat4x3<f32>(
        radiance[0] * expectation_sum_inv[0],
        radiance[1] * expectation_sum_inv[1],
        radiance[2] * expectation_sum_inv[2],
        radiance[3] * expectation_sum_inv[3],
    );
}

fn transmission(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> vec3<f32> {
    var transmission = vec3<f32>(1.0);

    let dim = vec3<f32>(grid(textureDimensions(VMM_OUT))) * f32(1u << CASCADE);
    let origin_sample = origin / dim;
    let direction_sample = direction / dim;

    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;

    let scale = length(TRANSFORM[0].xyz) * dim.x; // voxels/mm
    let step_size = 1.0 / radiance_scale;

    for (var t=t0; t<t1; t+=step_size) {
        let sample = origin_sample + direction_sample * t;

        let extinction = unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, sample, 0.0));

        transmission *= exp(-extinction * step_size / scale);
        if (all(transmission < vec3<f32>(1e-3)) || any(abs(sample - 0.5) > vec3<f32>(0.5))) { break; }
    }

    return transmission;
}

fn scatter_partial(k: u32, subgroup: u32, subgroup_index: u32, vmm: vec4<f32>, phi: vec4<f32>) {
    let vmm_sum = subgroupAdd(vmm);
    let phi_sum = subgroupAdd(phi);

    if (subgroup_index == 0u) {
        let i = SUBGROUPS * k + subgroup;
        VMM_PARTIAL[i] = vmm_sum;
        PHI_PARTIAL[i] = phi_sum;
    }
}

fn gather_partial(subgroup: u32, subgroup_index: u32) {
    let m = subgroup_index / SUBGROUPS;
    let s = subgroup_index % SUBGROUPS;
    let k = subgroup + m * SUBGROUPS;
    let i = SUBGROUPS * k + s;

    var vmm = VMM_PARTIAL[i];
    var phi = PHI_PARTIAL[i];

    for (var offset = 1u; offset < SUBGROUPS; offset *= 2u) {
        vmm += subgroupShuffleXor(vmm, offset);
        phi += subgroupShuffleXor(phi, offset);
    }

    if (s == 0u && k < VMM_SIZE) {
        let vmm_prior = vmm + 0.33 * VMM_PRIOR[k];

        let change = length(vmm_prior - VMM[k]);
        let scale = length(vmm_prior) + EPSILON;
        atomicMax(&VMM_DELTA, bitcast<u32>(change / scale));

        VMM[k] = vmm_prior;
        PHI[k] = phi;
    }
}

fn parent_uv(dim: vec3<u32>, probe: vec3<u32>, index: u32) -> vec3<f32> {
    let half = vec3<f32>(0.5);
    let parent_grid = vec3<f32>(grid(dim));
    let parent_probe = clamp(half * (vec3<f32>(probe) + half), half, parent_grid - half);
    return (parent_probe + parent_grid * vec3<f32>(lobe(index))) / vec3<f32>(dim);
}
