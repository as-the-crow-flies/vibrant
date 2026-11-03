// Workgroup-cooperative EM fit of the VMM to this workgroup's samples. Each
// including shader defines WORKGROUP, EM_ITERATIONS, vmm_size() (≤ VMM_MAX)
// and em_histogram_row().

use gaussian;
use random;
use sphere;

const VMM_MAX: u32 = 32u;

const EM_ITERATIONS_MAX: u32 = 100u;
const EM_ITERATIONS_MIN: u32 = 1u;
const EM_CONVERGENCE: f32 = 0.05;

struct Thread {
    @builtin(local_invocation_index) index: u32,
    @builtin(subgroup_id) subgroup: u32,
    @builtin(subgroup_invocation_id) lane: u32,
    @builtin(num_subgroups) subgroups: u32,
}

// One partial sum slot per 32 threads. Smaller subgroups (8, 16) share slots
// and take turns writing them, one round each.
const SLOTS: u32 = max(WORKGROUP / 32u, 1u);

var<workgroup> VMM: array<vec4<f32>, VMM_MAX>;
var<workgroup> VMM_PRIOR: array<vec4<f32>, VMM_MAX>;

var<workgroup> PHI: array<vec4<f32>, VMM_MAX>;

var<workgroup> SUM: array<vec4<f32>, VMM_MAX * SLOTS>;
var<workgroup> ROUNDS: u32;

var<workgroup> VMM_DELTA: atomic<u32>;
var<workgroup> VMM_CONVERGED: u32;

fn fit_vmm(omega: Directions, radiance: Radiances, weight: Weights, thread: Thread) {
    let lobes = vmm_size();

    var expectation = array<Weights, VMM_MAX>();
    var expectation_sum_inv = Weights();

    let rounds = subgroup_rounds(thread);

    var iteration = 1u;

    for (; iteration<=EM_ITERATIONS_MAX; iteration++) {
        if (thread.index == 0u) { atomicStore(&VMM_DELTA, 0u); }

        // Expectation
        var expectation_sum = Weights();

        for (var k=0u; k<lobes; k++) {
            expectation[k] = vmf(VMM[k], omega);
            expectation_sum += expectation[k];
        }

        expectation_sum_inv = 1.0 / max(expectation_sum, Weights(EPSILON));

        workgroupBarrier();

        // Maximization
        for (var round=0u; round<rounds; round++) {
            for (var k=0u; k<lobes; k++) {
                let gamma_weight = expectation[k] * expectation_sum_inv * weight;
                let vmm = vec4<f32>(omega * gamma_weight, total(gamma_weight));
                scatter_partial(k, round, thread, vmm);
            }

            workgroupBarrier();
        }

        if (thread.index < lobes) { update_vmm(thread.index, gather_partials(thread.index, thread)); }

        workgroupBarrier();

        if (is_converged(iteration, thread)) { break; }
    }

    if (thread.index == 0u) { record_iterations(iteration); }

    // Maximize Phi
    for (var round=0u; round<rounds; round++) {
        for (var k=0u; k<lobes; k++) {
            let gamma = expectation[k] * expectation_sum_inv;
            let phi = vec4<f32>(radiance * gamma, total(gamma));
            scatter_partial(k, round, thread, phi);
        }

        workgroupBarrier();
    }

    if (thread.index < lobes) { PHI[thread.index] = gather_partials(thread.index, thread); }

    workgroupBarrier();
}

fn is_converged(iteration: u32, thread: Thread) -> bool {
    if (thread.index == 0u) {
        let done = iteration >= EM_ITERATIONS_MIN && bitcast<f32>(atomicLoad(&VMM_DELTA)) < EM_CONVERGENCE;
        VMM_CONVERGED = select(0u, 1u, done);
    }

    return workgroupUniformLoad(&VMM_CONVERGED) != 0u;
}

fn update_vmm(k: u32, sum: vec4<f32>) {
    let vmm = sum + VMM_PRIOR[k];

    record_delta(VMM[k], vmm);

    VMM[k] = vmm;
}

fn record_delta(previous: vec4<f32>, next: vec4<f32>) {
    let change = length(next - previous);
    let scale = length(next) + EPSILON;
    atomicMax(&VMM_DELTA, bitcast<u32>(change / scale));
}

fn subgroup_rounds(thread: Thread) -> u32 {
    if (thread.index == 0u) { ROUNDS = (thread.subgroups + SLOTS - 1u) / SLOTS; }
    return workgroupUniformLoad(&ROUNDS);
}

fn scatter_partial(k: u32, round: u32, thread: Thread, item: vec4<f32>) {
    let sum = subgroupAdd(item);

    if (thread.lane == 0u && thread.subgroup / SLOTS == round) {
        let slot = SLOTS * k + thread.subgroup % SLOTS;
        if (round == 0u) { SUM[slot] = sum; } else { SUM[slot] += sum; }
    }
}

fn gather_partials(k: u32, thread: Thread) -> vec4<f32> {
    var sum = vec4<f32>(0.0);

    for (var slot = 0u; slot < min(thread.subgroups, SLOTS); slot++) {
        sum += SUM[SLOTS * k + slot];
    }

    return sum;
}

fn record_iterations(iterations: u32) {
    atomicAdd(&EM_ITERATIONS[em_histogram_row() * (EM_ITERATIONS_MAX + 1u) + iterations], 1u);
}

fn vmf(v: vec4<f32>, omega: Directions) -> Weights {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let k  = (3.0 * u2 - 1.0) / (u2 * u - u);          // Banerjee, reparameterized
    let kl = k * LOG2_E;

    let e    = exp2(fma(Weights(kl * q), project(v.xyz, omega), Weights(-kl))); // e^{κ(μ·ωᵢ − 1)}
    let norm = INV_TWO_PI * k / (1.0 - exp2(-2.0 * kl));
    return norm * e;
}

fn hammersley_direction(i: u32, N: u32) -> vec3<f32> {
    return octahedron_decode(2.0 * hammersley(i, N) - 1.0);
}

fn radiance_weight(radiance: vec3<f32>) -> f32 {
    return log(1.0 + length(radiance));
}
