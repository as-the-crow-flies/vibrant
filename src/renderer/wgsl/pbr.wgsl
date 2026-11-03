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

// PROPERTIES holds (Σ ior·w, Σ w, _) with w the material density. The epsilon
// fades to vacuum (ior 1) as density vanishes instead of amplifying 8-bit noise.
const AVERAGE_IOR_EPSILON: f32 = 1e-2;

fn average_ior(properties: vec3<f32>) -> f32 {
    return (properties.x + AVERAGE_IOR_EPSILON) / (properties.y + AVERAGE_IOR_EPSILON);
}
