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
const WORKGROUP_SIZE_SQRT: u32 = #WORKGROUP_SIZE_SQRT;

const HDRI_LOD_BIAS: f32 = 0.0;

var<workgroup> VMM: array<vec4<f32>, VMM_SIZE * 32u>; // Sufficient (w * omega, w)
var<workgroup> PHI: array<vec3<f32>, VMM_SIZE * 32u>; // Radiance & Distance (w * radiance, 1/d?)

// Resolution
// 1x1
// 2x2
// 4x4
// 8x8
// 16x16
// 32x32

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(num_workgroups) probes: vec3<u32>,
    @builtin(workgroup_id) probe: vec3<u32>,
    @builtin(local_invocation_index) local: u32,
    @builtin(subgroup_size) SUBGROUP_SIZE: u32,
    @builtin(subgroup_id) subgroup: u32,
    @builtin(subgroup_invocation_id) subgroup_index: u32,
    @builtin(num_subgroups) num_subgroups: u32,
) {
    if (local < VMM_SIZE) {
        let omega = octahedron_decode(hammersley(local, VMM_SIZE));
        VMM[local * SUBGROUP_SIZE] = vec4<f32>(omega / f32(VMM_SIZE), 1.0 / f32(VMM_SIZE));
    }

    let sample = vec2<u32>(local / WORKGROUP_SIZE_SQRT, local % WORKGROUP_SIZE_SQRT);

    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    let uv = 2.0 * (vec2<f32>(sample) + 0.5) / f32(WORKGROUP_SIZE_SQRT) - 1.0;
    let omega = octahedron_decode(uv);

    let radiance = hdri(omega, WORKGROUP_SIZE_SQRT);
    let weight = length(radiance);

    workgroupBarrier();

    var expectation = array<f32, VMM_SIZE>();

    for (var i=0u; i<100u; i++) {
        // Expectation
        var expectation_sum = 0.0;

        for (var k=0u; k<VMM_SIZE; k++) {
            let index = k * SUBGROUP_SIZE;
            expectation[k] = vmf(VMM[index], omega);
            expectation_sum += expectation[k];
        }

        let expectation_sum_inv = 1.0 / max(expectation_sum, EPSILON);

        workgroupBarrier();

        // Maximization
        // Compute Lobe Statistics
        for (var k=0u; k<VMM_SIZE; k++) {
            let sample_weight = weight * expectation[k] * expectation_sum_inv;

            let vmm = subgroupAdd(vec4<f32>(omega, 1.0) * sample_weight);
            let phi = subgroupAdd(radiance * sample_weight);

            if (subgroup_index == 0) {
                let index = num_subgroups * k + subgroup;
                VMM[index] = vmm;
                PHI[index] = phi;
            }
        }

        workgroupBarrier();

        // Reduce Lobe Statistics
        if (subgroup < VMM_SIZE) {
            let index = num_subgroups * subgroup + subgroup_index;

            let vmm = subgroupAdd(VMM[index]);
            let phi = subgroupAdd(PHI[index]);

            if (subgroup_index == 0) {
                let index = subgroup * SUBGROUP_SIZE;
                VMM[index] = vmm;
                PHI[index] = phi;
            }
        }

        workgroupBarrier();
    }

    if (local < VMM_SIZE) {
        let offset = probe + probes * vec3<u32>(local & 3, (local >> 2) & 3, (local >> 4) & 1);

        let index = local * SUBGROUP_SIZE;

        let gaussian = VMM[index];
        let radiance = vec4<f32>(PHI[index] / max(gaussian.w, EPSILON), 1.0);

        textureStore(GAUSSIAN_OUT, offset, gaussian);
        textureStore(RADIANCE_OUT, offset, radiance);
    }
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N * N)) + HDRI_LOD_BIAS;

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}
