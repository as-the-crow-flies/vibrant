fn van_der_corput(i: u32) -> f32 {
    return f32(reverseBits(i)) * 2.3283064365386963e-10; // 1 / 2^32
}

fn hammersley(i: u32, N: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(N), van_der_corput(i));
}

fn hammersley_rotated(i: u32, N: u32, offset: vec2<f32>) -> vec2<f32> {
    return fract(hammersley(i, N) + offset);
}

fn hash22(p: vec2<u32>) -> vec2<f32> {
    var v = p * 1664525u + 1013904223u;
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v.x ^= v.x >> 16u;
    v.y ^= v.y >> 16u;
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v.x ^= v.x >> 16u;
    v.y ^= v.y >> 16u;
    return vec2<f32>(v) * 2.3283064365386963e-10;
}

fn hash32(p: vec3<u32>) -> vec2<f32> {
    var v = p * 1664525u + 1013904223u;

    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;

    v ^= v >> vec3<u32>(16u);

    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;

    return vec2<f32>(v.xy) * 2.3283064365386963e-10;
}

// ── GGX Normal Distribution Function ──────────────────────────────────────
// α = roughness², n = surface normal, h = half-vector
fn D_GGX(n_dot_h: f32, alpha: f32) -> f32 {
    let a2     = alpha * alpha;
    let denom  = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom);
}

// ── Schlick-GGX (one side of Smith G) ─────────────────────────────────────
fn G_SchlickGGX(n_dot_v: f32, alpha: f32) -> f32 {
    // Use α²/2 remapping for analytical lights
    let k = (alpha * alpha) / 2.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

// ── Smith Geometry (shadowing + masking) ───────────────────────────────────
fn G_Smith(n_dot_v: f32, n_dot_l: f32, alpha: f32) -> f32 {
    let ggx_v = G_SchlickGGX(n_dot_v, alpha);
    let ggx_l = G_SchlickGGX(n_dot_l, alpha);
    return ggx_v * ggx_l;
}

// ── GGX Visibility (Heitz) ─────────────────────────────────────────────────
// Folds the Cook-Torrance denominator directly into the (separable) Smith
// masking-shadowing term, so V(NoL)*V(NoV) = G/(4*NoL*NoV). Used by the SG
// light-source specular fit - see
// https://therealmjp.github.io/posts/sg-series-part-4-specular-lighting-from-an-sg-light-source/
fn GGX_V1(n_dot_x: f32, alpha2: f32) -> f32 {
    return 1.0 / (n_dot_x + sqrt(alpha2 + (1.0 - alpha2) * n_dot_x * n_dot_x));
}

// ── Schlick Fresnel ────────────────────────────────────────────────────────
// f0 = base reflectance (vec3 for colored metals)
fn F_Schlick(v_dot_h: f32, f0: vec3<f32>) -> vec3<f32> {
    let fc = pow(1.0 - v_dot_h, 5.0);
    return f0 + (vec3(1.0) - f0) * fc;
}

// ── Full GGX Specular BRDF ─────────────────────────────────────────────────
fn GGX(
    normal: vec3<f32>,  // surface normal (normalized)
    view: vec3<f32>,    // direction toward camera (normalized)
    light: vec3<f32>,   // direction toward light  (normalized)
    roughness: f32,     // perceptual roughness
    f0: vec3<f32>       // reflectance at normal incidence
) -> vec3<f32> {
    let h = normalize(view + light);  // half-vector

    let n_dot_l = max(dot(normal, light), 0.0001);
    let n_dot_v = max(dot(normal, view),  0.0001);
    let n_dot_h = max(dot(normal, h),            0.0);
    let v_dot_h = max(dot(view, h),          0.0);

    // Remap perceptual roughness to α (Disney convention)
    let alpha = roughness * roughness;

    let D = D_GGX(n_dot_h, alpha);
    let G = G_Smith(n_dot_v, n_dot_l, alpha);
    let F = F_Schlick(v_dot_h, f0);

    // Cook-Torrance denominator
    let denom = 4.0 * n_dot_v * n_dot_l;
    return (D * G * F) / max(denom, 0.0001);
}

fn ggx_weight(normal: vec3<f32>, view: vec3<f32>, light: vec3<f32>, a2: f32) -> f32 {
    let n_dot_l = max(dot(normal, light), 0.0);
    let h = normalize(view + light);
    let n_dot_h = max(dot(normal, h), 0.0);
    let v_dot_h = max(dot(view, h), 0.0);

    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    let D = a2 / max(d * d, 1e-7);

    let F = 0.04 + 0.96 * pow(1.0 - v_dot_h, 5.0);

    return n_dot_l * D * F;
}

// Cook and Torrence 1981
fn fresnel(c: f32, eta: f32) -> f32 {
    let g2 = eta * eta + c * c - 1.0;

    if (g2 < 0.0) { return 1.0; } // total internal reflection

    let g = sqrt(g2);

    return pow(g - c, 2.0) / (2.0 * pow(g + c, 2.0)) *
        (1.0 + pow(c * (g + c) - 1.0, 2.0) / pow(c * (g - c) + 1.0, 2.0));
}

// Hemispherical (angle-independent) diffuse Fresnel reflectance Fdr(eta),
// the Egan-Hilgeman 1973 polynomial fit used by the classic dipole BSSRDF
// (Jensen et al. 2001) to weight light re-emerging from inside a medium.
// Unlike fresnel(c, eta), which spikes to 1 at grazing c -> 0 for a single
// direction, this integrates reflectance over the hemisphere, so it stays
// bounded and view-angle-independent - used to stop a per-sample specular
// Fresnel term from fully starving a diffuse/phase term at grazing angles.
fn fresnel_diffuse(eta: f32) -> f32 {
    // eta_boosted (the caller in trace.wgsl) can land on exactly 0 for
    // plausible IOR/specular-slider combinations; guard the reciprocal
    // rather than relying on 1/0 being well-defined across WGSL backends.
    let eta_safe = select(eta, 1e-4, abs(eta) < 1e-4);
    let inv_eta = 1.0 / eta_safe;

    // Fit is defined for eta >= 1; for eta < 1 evaluate it at 1/eta instead
    // (swap which side of the interface is "denser") rather than
    // extrapolating the polynomial outside its valid range. Clamped since
    // it's a polynomial fit, not a derivation - callers (e.g. eta_boosted in
    // trace.wgsl) can push eta outside [~1, ~3] where the fit is well
    // behaved, and a reflectance must stay in [0, 1] regardless.
    let fdr = select(
        -1.4399 * inv_eta * inv_eta + 0.7099 * inv_eta + 0.6681 + 0.0636 * eta,
        -1.4399 * eta * eta + 0.7099 * eta + 0.6681 + 0.0636 * inv_eta,
        eta < 1.0
    );

    return clamp(fdr, 0.0, 1.0);
}
