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

const VMM_SIZE: u32 = 8u;
const CASCADE: u32 = #CASCADE;
const WORKGROUP_SIZE: u32 = #WORKGROUP_SIZE;
const WORKGROUP_SIZE_SQRT: u32 = #WORKGROUP_SIZE_SQRT;
const EPSILON: f32 = 1e-6;

var<workgroup> SUFFICIENT: array<vec4<f32>, VMM_SIZE * 32u>; // (w * gamma * omega, w * gamma)
var<workgroup> AMPLITUDE: array<vec3<f32>, VMM_SIZE * 32u>; // AMPLITUDE per lobe
var<workgroup> VMM: array<vec4<f32>, VMM_SIZE>; // (mu * pi, kappa)

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
        let corner = normalize(vec3<f32>(
            select(-1.0, 1.0, (local & 1u) != 0u),
            select(-1.0, 1.0, (local & 2u) != 0u),
            select(-1.0, 1.0, (local & 4u) != 0u)
        ));

        VMM[local] = vec4<f32>(corner / f32(VMM_SIZE), 2.0);
    }

    let sample = vec2<u32>(local / WORKGROUP_SIZE_SQRT, local % WORKGROUP_SIZE_SQRT);

    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    let uv = 2.0 * (vec2<f32>(sample) + 0.5) / f32(WORKGROUP_SIZE_SQRT) - 1.0;
    let omega = octahedron_decode(uv);

    let radiance = hdri(omega, WORKGROUP_SIZE_SQRT);
    let weight = length(radiance);

    workgroupBarrier();

    for (var i=0u; i<10u; i++) {
        // Expectation
        var expectation = array<f32, VMM_SIZE>();
        var expectation_sum = 0.0;

        for (var k=0u; k<VMM_SIZE; k++) {
            expectation[k] = vmf(VMM[k], omega);
            expectation_sum += expectation[k];
        }

        let expectation_sum_inv = 1.0 / max(expectation_sum, EPSILON);

        // Maximization
        // Compute Lobe Statistics
        for (var k=0u; k<VMM_SIZE; k++) {
            let sample_weight = weight * expectation[k] * expectation_sum_inv;

            let sufficient = subgroupAdd(vec4<f32>(omega, 1.0) * sample_weight);
            let amplitude = subgroupAdd(radiance * sample_weight);

            if (subgroup_index == 0) {
                let index = num_subgroups * k + subgroup;
                SUFFICIENT[index] = sufficient;
                AMPLITUDE[index] = amplitude;
            }
        }

        workgroupBarrier();

        // Reduce Lobe Statistics
        if (subgroup < VMM_SIZE) {
            let index = num_subgroups * subgroup + subgroup_index;

            let sufficient = subgroupAdd(SUFFICIENT[index]);
            let amplitude = subgroupAdd(AMPLITUDE[index]);

            if (subgroup_index == 0) {
                let index = subgroup * SUBGROUP_SIZE;
                SUFFICIENT[index] = sufficient;
                AMPLITUDE[index] = amplitude;
            }
        }

        workgroupBarrier();

        // Sufficient to VMM
        if (subgroup == 0u) {
            var sufficient = vec4<f32>(0.0);
            if (subgroup_index < VMM_SIZE) {
                sufficient = SUFFICIENT[subgroup_index * SUBGROUP_SIZE];
            }
            let total = subgroupAdd(sufficient.w);
            if (subgroup_index < VMM_SIZE) {
                VMM[subgroup_index] = sufficient_to_vmf(sufficient, total);
            }
        }

        workgroupBarrier();
    }

    if (local < VMM_SIZE) {
        let offset = probe + probes * vec3<u32>(
            u32((local & 1u) != 0u),
            u32((local & 2u) != 0u),
            u32((local & 4u) != 0u));

        let weight_total = max(SUFFICIENT[local * SUBGROUP_SIZE].w, EPSILON);

        textureStore(GAUSSIAN_OUT, offset, VMM[local]);
        textureStore(RADIANCE_OUT, offset, vec4<f32>(AMPLITUDE[local * SUBGROUP_SIZE] / weight_total, 1.0));
    }
}

fn sufficient_to_vmf(sufficient: vec4<f32>, total: f32) -> vec4<f32> {
    let norm = max(length(sufficient.xyz), EPSILON);
    let r = clamp(norm / max(sufficient.w, EPSILON), 0.0, 0.9999);

    let mu = sufficient.xyz / norm;
    let pi = sufficient.w / max(total, EPSILON);
    let kappa = (3.0*r - r*r*r) / (1.0 - r*r);

    return vec4<f32>(mu * pi, kappa);
}

fn vmf(vmf: vec4<f32>, omega: vec3<f32>) -> f32 {
    let pi = length(vmf.xyz);
    let mu = vmf.xyz / max(pi, EPSILON);
    let kappa = vmf.w;

    return pi * kappa / (2.0 * PI * (1.0 - exp(-2.0 * kappa))) * exp(kappa * (dot(mu, omega) - 1.0));
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N * N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}
