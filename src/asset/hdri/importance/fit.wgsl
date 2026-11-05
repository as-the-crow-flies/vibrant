use em/em;
use em/samples_x4;
use uniforms;

@group(0) @binding(0) var<storage, read> CELLS: array<EnvironmentCell>;
@group(0) @binding(1) var<storage, read> SEED: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> VMM_OUT: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> PHI_OUT: array<vec4<f32>>;
@group(0) @binding(4) var<uniform> VMM_SIZE: u32;

// EM records iteration counts; the bake discards them.
var<workgroup> EM_ITERATIONS: array<atomic<u32>, EM_ITERATIONS_MAX + 1u>;

const WORKGROUP: u32 = 1024u;
const SAMPLES: u32 = 4096u;

fn vmm_size() -> u32 {
    return VMM_SIZE;
}

fn em_histogram_row() -> u32 {
    return 0u;
}

// Fits K lobes to the HDRI in world space at unit strength, from SEED.
// Like the seed, the fit for K lives at [K, 2K).
//
// Quadrature over the environment's cut: each sample is a cell's exact
// average, weighted by its solid angle, so no small bright feature falls
// between samples.
@compute
@workgroup_size(WORKGROUP)
fn main(thread: Thread) {
    if (thread.index < VMM_SIZE) { VMM[thread.index] = SEED[VMM_SIZE + thread.index]; }

    workgroupBarrier();

    let i = SAMPLES + SAMPLES_PER_THREAD * thread.index;
    let cells = array<EnvironmentCell, 4>(CELLS[i], CELLS[i + 1u], CELLS[i + 2u], CELLS[i + 3u]);

    let omega = sample_directions(thread);
    let radiance = Radiances(cells[0].color, cells[1].color, cells[2].color, cells[3].color);
    let weight = Weights(cells[0].weight, cells[1].weight, cells[2].weight, cells[3].weight);
    let ratio = Weights(cells[0].ratio, cells[1].ratio, cells[2].ratio, cells[3].ratio);

    fit_vmm(omega, radiance, weight, ratio, thread);

    if (thread.index < VMM_SIZE) {
        VMM_OUT[VMM_SIZE + thread.index] = VMM[thread.index];
        PHI_OUT[VMM_SIZE + thread.index] = PHI[thread.index];
    }
}

fn sample_direction(i: u32) -> vec3<f32> {
    return CELLS[SAMPLES + i].direction;
}
