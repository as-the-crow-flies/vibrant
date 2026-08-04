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

const INTERVAL = array<f32, 7>(0.0, 1.0, 3.0, 7.0, 15.0, 31.0, 63.0);

// LEVEL SAMPLES  THREADS  WORKGROUP
//     0       4        1         64
//     1      16        4         64
//     2      64       16         64
//     3     256       64         64
//     4    1024      256        256
//     5    4096     1024       1024

const CASCADE: u32 = #CASCADE;
const WORKGROUP_SIZE: u32 = #WORKGROUP_SIZE;
const SUBGROUPS: u32 = #SUBGROUPS;
const THREADS: u32 = #THREADS;
const SAMPLES: u32 = #SAMPLES;

const PROBES: u32 = WORKGROUP_SIZE / THREADS;

var<workgroup> VMM: array<vec4<f32>, PROBES * VMM_SIZE>;
var<workgroup> PHI: array<vec4<f32>, PROBES * VMM_SIZE>;

var<workgroup> VMM_PRIOR: array<vec4<f32>, PROBES * VMM_SIZE>;
var<workgroup> PHI_PRIOR: array<vec4<f32>, PROBES * VMM_SIZE>;

var<workgroup> VMM_PARTIAL: array<vec4<f32>, SUBGROUPS * VMM_SIZE>;
var<workgroup> PHI_PARTIAL: array<vec4<f32>, SUBGROUPS * VMM_SIZE>;

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
    // Initialize VMM
    for (var i = workgroup_index; i < PROBES * VMM_SIZE; i += WORKGROUP_SIZE) {
        let p = i / VMM_SIZE;
        let k = i % VMM_SIZE;

        let probe = vec3<u32>(workgroup.x * PROBES + p, workgroup.y, workgroup.z);

        let parent_dims = vec3<f32>(textureDimensions(GAUSSIAN_IN));
        let parent_grid = max(floor(parent_dims / vec3<f32>(4.0, 4.0, 2.0)), vec3<f32>(1.0));

        let parent_probe = clamp(
            0.5 * (vec3<f32>(probe) + 0.5),
            vec3<f32>(0.5),
            parent_grid - vec3<f32>(0.5));

        let parent_uv = (parent_probe + parent_grid * vec3<f32>(lobe(k))) / parent_dims;

        if (CASCADE == 5u) {
            VMM[i] = seed(probe, k);
        } else {
            VMM_PRIOR[i] = textureSampleLevel(GAUSSIAN_IN, SAMPLER, parent_uv, 0.0);
            PHI_PRIOR[i] = textureSampleLevel(RADIANCE_IN, SAMPLER, parent_uv, 0.0);

            VMM[i] = VMM_PRIOR[i];
        }
    }

    workgroupBarrier();

    let grid = vec3<u32>(textureDimensions(GAUSSIAN_OUT)) / vec3<u32>(4u, 4u, 2u);
    let p = workgroup_index / THREADS;
    let thread = workgroup_index % THREADS;
    let base = p * VMM_SIZE;

    let probe = vec3<u32>(workgroup.x * PROBES + p, workgroup.y, workgroup.z);
    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    let omega = mat4x3<f32>(
        get_direction(probe, 4u * thread + 0, SAMPLES),
        get_direction(probe, 4u * thread + 1, SAMPLES),
        get_direction(probe, 4u * thread + 2, SAMPLES),
        get_direction(probe, 4u * thread + 3, SAMPLES),
    );

    let radiance = mat4x3<f32>(
        hdri(omega[0], SAMPLES),
        hdri(omega[1], SAMPLES),
        hdri(omega[2], SAMPLES),
        hdri(omega[3], SAMPLES),
    );

    let weight = vec4<f32>(
        log2(1.0 + brightness(radiance[0])),
        log2(1.0 + brightness(radiance[1])),
        log2(1.0 + brightness(radiance[2])),
        log2(1.0 + brightness(radiance[3])),
    );

    var expectation = array<vec4<f32>, VMM_SIZE>();

    // Expectation Maximization
    for (var i=0u; i<20u; i++) {

        // Expectation
        var expectation_sum = vec4<f32>(0.0);

        for (var k=0u; k<VMM_SIZE; k++) {
            expectation[k] = vmf(VMM[base + k], omega);
            expectation_sum += expectation[k];
        }

        let expectation_sum_inv = 1.0 / max(expectation_sum, vec4<f32>(EPSILON));

        workgroupBarrier();

        // Maximization
        for (var k=0u; k<VMM_SIZE; k++) {
            let gamma = expectation[k] * expectation_sum_inv;
            let gamma_weight = gamma * weight;

            var vmm = vec4<f32>(omega * gamma_weight, sum(gamma_weight));
            var phi = vec4<f32>(radiance * gamma, sum(gamma));

            if (PROBES > 1u) {
                for (var offset = 1u; offset < THREADS; offset *= 2u) {
                    vmm += subgroupShuffleXor(vmm, offset);
                    phi += subgroupShuffleXor(phi, offset);
                }

                if (thread == 0u) {
                    store_lobe(base + k, vmm, phi);
                }
            } else {
                vmm = subgroupAdd(vmm);
                phi = subgroupAdd(phi);

                if (subgroup_index == 0) {
                    let index = SUBGROUPS * k + subgroup;
                    VMM_PARTIAL[index] = vmm;
                    PHI_PARTIAL[index] = phi;
                }
            }
        }

        workgroupBarrier();

        if (PROBES == 1u) {
            for (var k=subgroup; k<VMM_SIZE; k+=SUBGROUPS) {
                let index = SUBGROUPS * k + subgroup_index;

                let vmm = subgroupAdd(select(vec4<f32>(0.0), VMM_PARTIAL[index], subgroup_index < SUBGROUPS));
                let phi = subgroupAdd(select(vec4<f32>(0.0), PHI_PARTIAL[index], subgroup_index < SUBGROUPS));

                if (subgroup_index == 0) {
                    store_lobe(k, vmm, phi);
                }
            }

            workgroupBarrier();
        }
    }

    for (var i = workgroup_index; i < PROBES * VMM_SIZE; i += WORKGROUP_SIZE) {
        let p = i / VMM_SIZE;
        let k = i % VMM_SIZE;

        let voxel = vec3<u32>(workgroup.x * PROBES + p, workgroup.y, workgroup.z);

        let offset = voxel + grid * lobe(k);

        textureStore(GAUSSIAN_OUT, offset, VMM[i]);
        textureStore(RADIANCE_OUT, offset, norm(PHI[i]));
    }
}

fn store_lobe(k: u32, vmm: vec4<f32>, phi: vec4<f32>) {
    let factor = ENVIRONMENT.settings.ambient_light;

    VMM[k] = vmm + factor * VMM_PRIOR[k];
    PHI[k] = phi + factor * PHI_PRIOR[k];
}

fn get_radiance(direction: vec3<f32>, N: u32) -> vec4<f32> {
    let radiance = hdri(direction, N);
    return vec4<f32>(radiance, length(radiance));
}

fn get_direction(probe: vec3<u32>, i: u32, N: u32) -> vec3<f32> {
    let uv = fract(hammersley(i, N));
    return octahedron_decode(2.0 * uv - 1.0);
}

fn seed(probe: vec3<u32>, k: u32) -> vec4<f32> {
    let rotation = rotation_y(-HDRI_SETTINGS.rotation * 2.0 * PI);
    let omega = rotation * get_direction(probe, k, VMM_SIZE);
    return vec4<f32>(omega, 1.0) / f32(VMM_SIZE);
}

fn sum(v: vec4<f32>) -> f32 {
    return v.x + v.y + v.z + v.w;
}

fn norm(v: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(v.rgb / max(v.w, EPSILON), 1.0);
}

fn lobe(k: u32) -> vec3<u32> {
    return vec3<u32>(k & 3, (k >> 2) & 3, (k >> 4) & 1);
}
