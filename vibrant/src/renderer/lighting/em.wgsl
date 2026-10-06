// Workgroup-cooperative EM fit of the VMM to this workgroup's samples. Each
// including shader defines SUBGROUPS, EM_HISTOGRAM_ROW and the EM_ITERATIONS binding.

const VMM_SIZE: u32 = #VMM_SIZE;

const EM_ITERATIONS_MAX: u32 = 100u;
const EM_ITERATIONS_MIN: u32 = 1u;
const EM_CONVERGENCE: f32 = 0.05;

struct Thread {
    @builtin(local_invocation_index) index: u32,
    @builtin(subgroup_id) subgroup: u32,
    @builtin(subgroup_invocation_id) lane: u32,
}

var<workgroup> VMM: array<vec4<f32>, VMM_SIZE>;
var<workgroup> VMM_PRIOR: array<vec4<f32>, VMM_SIZE>;

var<workgroup> PHI: array<vec4<f32>, VMM_SIZE>;

var<workgroup> SUM: array<vec4<f32>, VMM_SIZE * SUBGROUPS>;

var<workgroup> VMM_DELTA: atomic<u32>;
var<workgroup> VMM_CONVERGED: u32;

fn fit_vmm(omega: Directions, radiance: Radiances, weight: Weights, thread: Thread) {
    var expectation = array<Weights, VMM_SIZE>();
    var expectation_sum_inv = Weights();

    var iteration = 1u;

    for (; iteration<=EM_ITERATIONS_MAX; iteration++) {
        if (thread.index == 0u) { atomicStore(&VMM_DELTA, 0u); }

        // Expectation
        var expectation_sum = Weights();

        for (var k=0u; k<VMM_SIZE; k++) {
            expectation[k] = vmf(VMM[k], omega);
            expectation_sum += expectation[k];
        }

        expectation_sum_inv = 1.0 / max(expectation_sum, Weights(EPSILON));

        workgroupBarrier();

        // Maximization
        for (var k=0u; k<VMM_SIZE; k++) {
            let gamma_weight = expectation[k] * expectation_sum_inv * weight;
            let vmm = vec4<f32>(omega * gamma_weight, total(gamma_weight));
            scatter_partial(k, thread, vmm);
        }

        workgroupBarrier();

        if (thread.index < VMM_SIZE) { update_vmm(thread.index, gather_partials(thread.index)); }

        workgroupBarrier();

        if (is_converged(iteration, thread)) { break; }
    }

    if (thread.index == 0u) { record_iterations(iteration); }

    // Maximize Phi
    for (var k=0u; k<VMM_SIZE; k++) {
        let gamma = expectation[k] * expectation_sum_inv;
        let phi = vec4<f32>(radiance * gamma, total(gamma));
        scatter_partial(k, thread, phi);
    }

    workgroupBarrier();

    if (thread.index < VMM_SIZE) { PHI[thread.index] = gather_partials(thread.index); }

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

fn scatter_partial(k: u32, thread: Thread, item: vec4<f32>) {
    let sum = subgroupAdd(item);

    if (thread.lane == 0u) {
        SUM[SUBGROUPS * k + thread.subgroup] = sum;
    }
}

fn gather_partials(k: u32) -> vec4<f32> {
    var sum = vec4<f32>(0.0);

    for (var subgroup = 0u; subgroup < SUBGROUPS; subgroup++) {
        sum += SUM[SUBGROUPS * k + subgroup];
    }

    return sum;
}

fn record_iterations(iterations: u32) {
    atomicAdd(&EM_ITERATIONS[EM_HISTOGRAM_ROW * (EM_ITERATIONS_MAX + 1u) + iterations], 1u);
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
