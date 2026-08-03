const VMM_SIZE   : u32 = 16u;
const EPSILON    : f32 = 1e-6;
const INV_TWO_PI : f32 = 0.15915494309189535;
const LOG2_E     : f32 = 1.4426950408889634;
const U_MIN      : f32 = 1.001;    // κ_max ≈ 1000
const U_MAX      : f32 = 100.0;    // κ_min ≈ 0.03 — indistinguishable from uniform

fn vmf(v: vec4<f32>, omega: mat4x3<f32>) -> vec4<f32> {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let k  = (3.0 * u2 - 1.0) / (u2 * u - u);          // Banerjee, reparameterized
    let kl = k * LOG2_E;

    let e = exp2(fma(vec4<f32>(kl * q), transpose(omega) * v.xyz, vec4<f32>(-kl))); // e^{κ(μ·ωᵢ − 1)}
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

// Approximates the outgoing specular radiance from a single SG/vMF light
// lobe L(ω) = phi·exp(κ·(μ·ω − 1)) (same sufficient statistic `v` as vmf()
// and sg_irradiance_fitted()) reflected by a GGX surface, following the
// SG-light-source technique from
// https://therealmjp.github.io/posts/sg-series-part-4-specular-lighting-from-an-sg-light-source/
// (itself building on Wang et al., "All-Frequency Rendering with Dynamic,
// Spatially Varying Reflectance"):
//
//  - The GGX NDF is represented as an SG in half-vector space, centered on N
//    with sharpness λ_ndf = 2/α² and amplitude 1/(πα²) (matches D_GGX at its
//    peak, H=N).
//  - It's warped into light-vector space by re-centering on the reflection
//    vector R = reflect(-V,N) and dividing its sharpness by 4|N·V| (the
//    half-vector→light-vector Jacobian; amplitude is unchanged). That turns
//    "integrate the NDF against the light lobe" into a product of two SGs,
//    which has the same closed-form sphere integral used above
//    (SGInnerProduct).
//  - Visibility (Heitz, folding G/(4·NoL·NoV) into one term) and Fresnel are
//    then applied at the lobe's peak direction, where N·L = N·V and the
//    half-vector reduces to N — matching the reference's assumption that
//    both terms are ~constant across the BRDF lobe.
fn sg_specular_fitted(phi: vec3<f32>, v: vec4<f32>, view: vec3<f32>, normal: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let q     = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);
    let u     = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);
    let u2    = u * u;
    let kappa = (3.0 * u2 - 1.0) / (u2 * u - u);
    let mu    = v.xyz * q;

    let n_dot_v    = max(dot(normal, view), EPSILON);
    let reflection = reflect(-view, normal); // warped NDF axis

    let alpha  = roughness * roughness;
    let alpha2 = max(alpha * alpha, 1e-8);

    let lambda_r = 0.5 / (alpha2 * n_dot_v); // = (2/α²) / (4·n_dot_v)

    let cos_mu_r = dot(mu, reflection);
    let delta    = kappa * kappa + 2.0 * kappa * lambda_r * cos_mu_r; // λ3² - λ_r², exact
    let lambda3  = sqrt(delta + lambda_r * lambda_r);

    let exponent = delta / (lambda3 + lambda_r) - kappa;

    let ratio = lambda_r / lambda3;
    let scale = exp2(exponent * LOG2_E);
    let em2l3 = exp2(-2.0 * lambda3 * LOG2_E);

    let inner_product = phi * (4.0 * n_dot_v) * ratio * scale * (1.0 - em2l3);

    let visibility = GGX_V1(n_dot_v, alpha2) * GGX_V1(n_dot_v, alpha2); // N·L = N·V at the peak
    let F = F_Schlick(n_dot_v, f0); // V·H = N·V at the peak (H = N)

    return inner_product * visibility * F * n_dot_v; // cosine term, N·L = n_dot_v
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}
