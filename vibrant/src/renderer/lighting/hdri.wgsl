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

const EM_HISTOGRAM_ROW: u32 = 6u; // == GaussianRadianceBuffer::LEVELS

@compute
@workgroup_size(1024)
fn main(thread: Thread) {
    if (thread.index < VMM_SIZE) { initialize_lobe(thread.index); }

    workgroupBarrier();

    let omega = hammersley_directions(thread);
    let radiance = trace_samples(vec3<f32>(0.0), omega);
    let weight = radiance_weights(radiance);

    fit_vmm(omega, radiance, weight, thread);

    if (thread.index < VMM_SIZE) { store_lobe(thread.index); }
}

fn initialize_lobe(k: u32) {
    VMM[k] = vec4<f32>(hammersley_direction(k, VMM_SIZE), 1.0) / f32(VMM_SIZE);
}

fn store_lobe(k: u32) {
    let vmm = VMM[k];

    let rotated = rotate_hdri(vmm.xyz) * length(vmm.xyz);

    VMM_HDRI_OUT[k] = vec4<f32>(rotated, vmm.w);
    PHI_HDRI_OUT[k] = PHI[k];
}

fn rotate_hdri(direction: vec3<f32>) -> vec3<f32> {
    let world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let world_rotated = rotation_z(-HDRI_SETTINGS.rotation * 2.0 * PI) * world;
    let local = normalize((TRANSFORM * vec4<f32>(world_rotated, 0.0)).xyz);

    return local;
}

fn trace(origin: vec3<f32>, direction: vec3<f32>) -> vec3<f32> {
    return hdri(direction);
}

fn hdri(direction: vec3<f32>) -> vec3<f32> {
    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, 0.0);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, 0.0).rgb;

    return radiance;
}
