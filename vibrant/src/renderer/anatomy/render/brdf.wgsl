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

// ── Schlick Fresnel ────────────────────────────────────────────────────────
// f0 = base reflectance (vec3 for colored metals)
fn F_Schlick(v_dot_h: f32, f0: vec3<f32>) -> vec3<f32> {
    let fc = pow(1.0 - v_dot_h, 5.0);
    return f0 + (vec3(1.0) - f0) * fc;
}

// ── Full GGX Specular BRDF ─────────────────────────────────────────────────
struct BRDFInput {
    normal:    vec3<f32>,   // surface normal (normalized)
    view_dir:  vec3<f32>,   // direction toward camera (normalized)
    light_dir: vec3<f32>,   // direction toward light  (normalized)
    roughness: f32,         // perceptual roughness (usually remapped: α = r²)
    f0:        vec3<f32>,   // reflectance at normal incidence
}

fn GGX_Specular(b: BRDFInput) -> vec3<f32> {
    let h = normalize(b.view_dir + b.light_dir);  // half-vector

    let n_dot_l = max(dot(b.normal, b.light_dir), 0.0001);
    let n_dot_v = max(dot(b.normal, b.view_dir),  0.0001);
    let n_dot_h = max(dot(b.normal, h),            0.0);
    let v_dot_h = max(dot(b.view_dir, h),          0.0);

    // Remap perceptual roughness to α (Disney convention)
    let alpha = b.roughness * b.roughness;

    let D = D_GGX(n_dot_h, alpha);
    let G = G_Smith(n_dot_v, n_dot_l, alpha);
    let F = F_Schlick(v_dot_h, b.f0);

    // Cook-Torrance denominator
    let denom = 4.0 * n_dot_v * n_dot_l;
    return (D * G * F) / max(denom, 0.0001);
}
