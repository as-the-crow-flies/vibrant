@group(0) @binding(0) var PHI_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(1) var VMM_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var PHI_IN: texture_3d<f32>;
@group(0) @binding(4) var VMM_IN: texture_3d<f32>;

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

var<workgroup> VMM: array<vec4<f32>, VMM_SIZE>;
var<workgroup> PHI: array<vec4<f32>, VMM_SIZE>;

var<workgroup> VMM_PRIOR: array<vec4<f32>, VMM_SIZE>;
var<workgroup> PHI_PRIOR: array<vec4<f32>, VMM_SIZE>;

var<workgroup> VMM_PARTIAL: array<vec4<f32>, VMM_SIZE * SUBGROUPS>;
var<workgroup> PHI_PARTIAL: array<vec4<f32>, VMM_SIZE * SUBGROUPS>;

@compute
@workgroup_size(WORKGROUP)
fn main(
    @builtin(workgroup_id) probe: vec3<u32>,
    @builtin(local_invocation_index) index: u32,
) {
    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    if (cull(origin)) { return; }

    if (index < VMM_SIZE) { initialize(probe, index); }

    workgroupBarrier();

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
}

fn cull(origin: vec3<f32>) -> bool {
    return false;
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

    for (var i=0u; i<20u; i++) {

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

            let vmm = subgroupAdd(vec4<f32>(omega * gamma_weight, sum(gamma_weight)));
            let phi = subgroupAdd(vec4<f32>(radiance * gamma, sum(gamma)));

            if (subgroup_index == 0u) {
                let i = SUBGROUPS * k + subgroup;
                VMM_PARTIAL[i] = vmm;
                PHI_PARTIAL[i] = phi;
            }
        }

        workgroupBarrier();

        for (var k = subgroup; k < VMM_SIZE; k += SUBGROUPS) {
            let i = SUBGROUPS * k + subgroup_index;

            let vmm = subgroupAdd(select(vec4<f32>(0.0), VMM_PARTIAL[i], subgroup_index < SUBGROUPS));
            let phi = subgroupAdd(select(vec4<f32>(0.0), PHI_PARTIAL[i], subgroup_index < SUBGROUPS));

            if (subgroup_index == 0u) {
                VMM[k] = vmm + 0.33 * VMM_PRIOR[k];
                PHI[k] = phi + 0.33 * PHI_PRIOR[k];
            }
        }

        workgroupBarrier();
    }
}

fn store(probe: vec3<u32>, index: u32) {
    let texel = probe + grid(textureDimensions(VMM_OUT)) * lobe(index);

    let vmm = VMM[index];
    let phi = max_norm(PHI[index]);

    textureStore(VMM_OUT, texel, vmm);
    textureStore(PHI_OUT, texel, phi);
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

        radiance += mat4x3<f32>(
            expectation[0] * phi.rgb / phi.w,
            expectation[1] * phi.rgb / phi.w,
            expectation[2] * phi.rgb / phi.w,
            expectation[3] * phi.rgb / phi.w,
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

    let radiance_scale = f32(textureDimensions(ABSORPTION).x) / dim.x;

    let scale = length(TRANSFORM[0].xyz) * dim.x; // voxels/mm
    let step_size = 1.0 / radiance_scale;

    for (var t=t0; t<t1; t+=step_size) {
        let sample = origin_sample + direction_sample * t;

        let extinction =
            unpack_rgb(textureSampleLevel(ABSORPTION, SAMPLER, sample, 0.0)) +
            unpack_rgb(textureSampleLevel(SCATTERING, SAMPLER, sample, 0.0));

        transmission *= exp(-extinction * step_size / scale);
        if (all(transmission < vec3<f32>(1e-3)) || any(abs(sample - 0.5) > vec3<f32>(0.5))) { break; }
    }

    return transmission;
}

fn parent_uv(dim: vec3<u32>, probe: vec3<u32>, index: u32) -> vec3<f32> {
    let half = vec3<f32>(0.5);
    let parent_grid = vec3<f32>(grid(dim));
    let parent_probe = clamp(half * (vec3<f32>(probe) + half), half, parent_grid - half);
    return (parent_probe + parent_grid * vec3<f32>(lobe(index))) / vec3<f32>(dim);
}
