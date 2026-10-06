@group(0) @binding(0) var HDRI: texture_2d<f32>;
@group(0) @binding(1) var HDRI_SAMPLER: sampler;
@group(0) @binding(2) var<storage, read> SEED: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> VMM_OUT: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> PHI_OUT: array<vec4<f32>>;

// EM records iteration counts; the bake discards them.
var<workgroup> EM_ITERATIONS: array<atomic<u32>, EM_ITERATIONS_MAX + 1u>;

const SUBGROUPS: u32 = 32u;
const SAMPLES: u32 = 4096u;

const EM_HISTOGRAM_ROW: u32 = 0u;

// Fits K lobes to the HDRI in world space at unit strength, from SEED.
// Like the seed, the fit for K lives at [K, 2K).
@compute
@workgroup_size(1024)
fn main(thread: Thread) {
    if (thread.index < VMM_SIZE) { VMM[thread.index] = SEED[VMM_SIZE + thread.index]; }

    workgroupBarrier();

    let omega = hammersley_directions(thread);
    let radiance = trace_samples(vec3<f32>(0.0), omega);
    let weight = radiance_weights(radiance);

    fit_vmm(omega, radiance, weight, thread);

    if (thread.index < VMM_SIZE) {
        VMM_OUT[VMM_SIZE + thread.index] = VMM[thread.index];
        PHI_OUT[VMM_SIZE + thread.index] = PHI[thread.index];
    }
}

fn trace(origin: vec3<f32>, direction: vec3<f32>) -> vec3<f32> {
    return textureSampleLevel(HDRI, HDRI_SAMPLER, equirectangular(direction.xzy, 0.0), 0.0).rgb;
}
