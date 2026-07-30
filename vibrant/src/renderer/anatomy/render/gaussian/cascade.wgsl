@group(0) @binding(0) var RADIANCE_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(1) var GAUSSIAN_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var RADIANCE_IN: texture_3d<f32>;
@group(0) @binding(4) var GAUSSIAN_IN: texture_3d<f32>;

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

const INTERVAL = array<f32, 11>(0.0, 1.0, 3.0, 7.0, 15.0, 31.0, 63.0, 127.0, 255.0, 511.0, 1023.0);

const CASCADE: u32 = #CASCADE;
const WORKGROUP_SIZE: u32 = #WORKGROUP_SIZE;
const SUBGROUPS: u32 = #SUBGROUPS;
const SAMPLES: u32 = #SAMPLES;

var<workgroup> VMM: array<vec4<f32>, VMM_SIZE>; // Final per-lobe (w * omega, w)
var<workgroup> PHI: array<vec3<f32>, VMM_SIZE>; // Final per-lobe (w * radiance, 1/d?)

var<workgroup> VMM_PARTIAL: array<vec4<f32>, SUBGROUPS * VMM_SIZE>;
var<workgroup> PHI_PARTIAL: array<vec3<f32>, SUBGROUPS * VMM_SIZE>;

// Cascade  Samples Subgroups   Workgroup Size
// 0        128     1           32
// 1        128     1           32
// 2        128     1           32
// 3        256     2           64
// 4        1024    8           256
// 5        4096    32          1024

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
    @builtin(workgroup_id) workgroup: vec3<u32>,
    @builtin(subgroup_id) subgroup: u32,
    @builtin(local_invocation_index) workgroup_index: u32,
    @builtin(subgroup_invocation_id) subgroup_index: u32,
) {
    // TODO: Read from last frame! (and from previous cascade, obviously)
    if (workgroup_index < VMM_SIZE) {
        let omega = get_direction(workgroup_index, VMM_SIZE);
        VMM[workgroup_index] = vec4<f32>(omega / f32(VMM_SIZE), 1.0 / f32(VMM_SIZE));
    }

    let origin = (vec3<f32>(workgroup) + 0.5) * f32(1u << CASCADE);

    let omega = mat4x3<f32>(
        get_direction(4u * workgroup_index + 0, SAMPLES),
        get_direction(4u * workgroup_index + 1, SAMPLES),
        get_direction(4u * workgroup_index + 2, SAMPLES),
        get_direction(4u * workgroup_index + 3, SAMPLES)
    );

    let radiance = mat4x3<f32>(
        hdri(omega[0], SAMPLES),
        hdri(omega[1], SAMPLES),
        hdri(omega[2], SAMPLES),
        hdri(omega[3], SAMPLES),
    );

    let weight = vec4<f32>(
        length(radiance[0]),
        length(radiance[1]),
        length(radiance[2]),
        length(radiance[3]),
    );

    workgroupBarrier();

    var expectation = array<vec4<f32>, VMM_SIZE>();

    for (var i=0u; i<50u; i++) {

        // Expectation
        var expectation_sum = vec4<f32>(0.0);

        for (var k=0u; k<VMM_SIZE; k++) {
            let gamma = vmf(VMM[k], omega);

            expectation[k] = weight * gamma;
            expectation_sum += gamma;
        }

        let expectation_sum_inv = 1.0 / max(expectation_sum, vec4<f32>(EPSILON));

        workgroupBarrier();

        // Maximization
        for (var k=0u; k<VMM_SIZE; k++) {
            let sample_weight = expectation[k] * expectation_sum_inv;

            let vmm_thread = vec4<f32>(omega * sample_weight, sum(sample_weight));
            let phi_thread = radiance * sample_weight;

            // Reduce Subgroup
            let vmm = subgroupAdd(vmm_thread);
            let phi = subgroupAdd(phi_thread);

            if (subgroup_index == 0) {
                let index = SUBGROUPS * k + subgroup;
                VMM_PARTIAL[index] = vmm;
                PHI_PARTIAL[index] = phi;
            }
        }

        workgroupBarrier();

        for (var k=subgroup; k<VMM_SIZE; k+=SUBGROUPS) {
            let index = SUBGROUPS * k + subgroup_index;

            let vmm = subgroupAdd(select(vec4<f32>(0.0), VMM_PARTIAL[index], subgroup_index < SUBGROUPS));
            let phi = subgroupAdd(select(vec3<f32>(0.0), PHI_PARTIAL[index], subgroup_index < SUBGROUPS));

            if (subgroup_index == 0) {
                VMM[k] = vmm;
                PHI[k] = phi;
            }
        }

        workgroupBarrier();
    }

    if (workgroup_index < VMM_SIZE) {
        let offset = workgroup + num_workgroups * vec3<u32>(workgroup_index & 3, (workgroup_index >> 2) & 3, (workgroup_index >> 4) & 1);

        let gaussian = VMM[workgroup_index];
        let radiance = vec4<f32>(PHI[workgroup_index] / max(gaussian.w, EPSILON), 1.0);

        textureStore(GAUSSIAN_OUT, offset, gaussian);
        textureStore(RADIANCE_OUT, offset, radiance);
    }
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}

fn get_radiance(direction: vec3<f32>, N: u32) -> vec4<f32> {
    let radiance = hdri(direction, N);
    return vec4<f32>(radiance, length(radiance));
}

fn get_direction(i: u32, N: u32) -> vec3<f32> {
    return octahedron_decode(2.0 * hammersley(i, N) - 1.0);
}

fn sum(v: vec4<f32>) -> f32 {
    return v.x + v.y + v.z + v.w;
}
