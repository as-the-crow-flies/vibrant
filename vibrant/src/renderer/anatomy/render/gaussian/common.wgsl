const VMM_SIZE   : u32 = 32u;
const EPSILON    : f32 = 1e-6;
const INV_TWO_PI : f32 = 0.15915494309189535;
const LOG2_E     : f32 = 1.4426950408889634;
const U_MIN      : f32 = 1.001;    // κ_max ≈ 1000
const U_MAX      : f32 = 100.0;    // κ_min ≈ 0.03 — indistinguishable from uniform

fn vmf(v: vec4<f32>, omega: vec3<f32>) -> f32 {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let k  = (3.0 * u2 - 1.0) / (u2 * u - u);          // Banerjee, reparameterized
    let kl = k * LOG2_E;

    let e = exp2(fma(kl * q, dot(v.xyz, omega), -kl)); // e^{κ(μ·ω − 1)}
    return INV_TWO_PI * k * e / (1.0 - exp2(-2.0 * kl));
}

// Fitted analytic approximation of ∫_hemisphere(N) L(ω)⟨N·ω⟩dω for a single
// SG/vMF lobe L(ω) = phi·exp(κ·(μ·ω − 1)), where `v` = (Σwᵢωᵢ, Σwᵢ) is the same
// sufficient statistic consumed by vmf() — κ and μ are recovered from it directly
// instead of requiring the caller to normalize/derive them up front.
// See: https://therealmjp.github.io/posts/sg-series-part-3-diffuse-lighting-from-an-sg-light-source/
//
// The published fit diverges for κ ≲ 1 (it targets tight specular-like lobes), so the
// lobe shape is evaluated at a clamped sharpness and blended towards the exact
// isotropic-lobe irradiance (π·phi) as κ → 0. Near-isotropic/degenerate statistics
// (v.xyz ≈ 0) fall out of the same κ-clamp used by vmf(), so no extra branch is needed.
fn sg_irradiance_fitted(phi: vec3<f32>, v: vec4<f32>, normal: vec3<f32>) -> vec3<f32> {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let kappa  = (3.0 * u2 - 1.0) / (u2 * u - u);      // Banerjee, reparameterized
    let lambda = max(kappa, 1.0);

    let mu_dot_n = clamp(dot(v.xyz, normal) * q, -1.0, 1.0);

    let c0 = 0.36;
    let c1 = 1.0 / (4.0 * c0);

    let eml  = exp2(-lambda * LOG2_E);
    let em2l = eml * eml;
    let rl   = 1.0 / lambda;

    let scale = 1.0 + 2.0 * em2l - rl;
    let bias  = (eml - em2l) * rl - em2l;

    let x  = sqrt(max(1.0 - scale, 0.0));
    let x0 = c0 * mu_dot_n;
    let x1 = c1 * x;

    let n = x0 + x1;

    var y = clamp(mu_dot_n, 0.0, 1.0);
    if (abs(x0) <= x1) {
        y = n * n / max(x, EPSILON);
    }

    let result = scale * y + bias;

    // Exact SG integral over the sphere is 2π·phi·(1 − e^{−2λ})/λ; em2l is already
    // available from scale/bias above, so this reuses it instead of a third exp call.
    let fitted = result * (2.0 * PI * phi * (1.0 - em2l) / lambda);

    return mix(PI * phi, fitted, clamp(kappa, 0.0, 1.0));
}

// Approximates ∫ L(ω) f_GGX(ω, V, N) (N·ω) dω for a single SG/vMF light lobe
// L(ω) = phi·exp(κ·(μ·ω − 1)) (same sufficient statistic `v` as vmf() and
// sg_irradiance_fitted()) against the full Cook-Torrance GGX specular BRDF.
//
// The GGX NDF is itself represented as an SG lobe: in half-vector space it is
// centered on N with sharpness λ_ndf = 2/α² and amplitude 1/(πα²) (the
// standard SG fit to D_GGX, see Wang et al. "All-Frequency Rendering with
// Dynamic, Spatially Varying Reflectance"). It is then warped into light-
// vector space — centered on the reflection vector R = reflect(-V,N), with
// sharpness divided by 4·(N·V) to account for the half-vector→light-vector
// Jacobian — which turns the light-integral into a product of two SGs, and
// that product has the same closed-form sphere integral used above. F and G
// are treated as constant over the lobe and evaluated at its peak (L=R, so
// H=N and V·H reduces to N·V).
fn sg_specular_fitted(phi: vec3<f32>, v: vec4<f32>, view: vec3<f32>, normal: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let q     = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);
    let u     = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);
    let u2    = u * u;
    let kappa = (3.0 * u2 - 1.0) / (u2 * u - u);
    let mu    = v.xyz * q;

    let n_dot_v      = max(dot(normal, view), EPSILON);
    let reflection   = reflect(-view, normal);

    let alpha = roughness * roughness;
    let a2    = max(alpha * alpha, 1e-8);

    // amplitude_ndf = 1/(π·α²) and λ_r = 1/(2·α²·n_dot_v) both blow up as
    // roughness → 0, and the closed-form integral wants amplitude_ndf/λ3 —
    // computing that as (huge amplitude) × (1/huge λ3) loses almost all f32
    // precision right where the lobe gets sharp, which is exactly what showed
    // up as whiteout/noise at low roughness. Substituting
    // amplitude_ndf = 4·n_dot_v·λ_r/(2π) (exact, from the α² definitions
    // above) turns that product into 4·n_dot_v·(λ_r/λ3), a ratio of two
    // same-magnitude quantities — and the 4·n_dot_v then cancels the
    // Cook-Torrance denominator's 4·n_dot_v below, so it never needs to be
    // formed at all.
    let lambda_r = 0.5 / (a2 * n_dot_v); // = (2/a2) / (4·n_dot_v)

    let d       = kappa * mu + lambda_r * reflection;
    let lambda3 = max(length(d), EPSILON);

    let ratio = lambda_r / lambda3; // both O(λ_r): stable, bounded in (0,1]
    let scale = exp2((lambda3 - kappa - lambda_r) * LOG2_E);
    let em2l3 = exp2(-2.0 * lambda3 * LOG2_E);

    let shape = ratio * scale * (1.0 - em2l3);

    let G = G_Smith(n_dot_v, n_dot_v, alpha); // N·L = N·V at the lobe peak
    let F = F_Schlick(n_dot_v, f0);

    return phi * shape * G * F;
}
