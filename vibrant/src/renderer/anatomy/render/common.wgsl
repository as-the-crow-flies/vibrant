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

fn octahedron_inverse(direction: vec3<f32>, count: u32) -> vec2<u32> {
    var bary = abs(direction) / (abs(direction.x) + abs(direction.y) + abs(direction.z));
    var sub = vec2<u32>(0u);

    var half = count >> 1u;
    while (half > 0u) {
        let bit = countTrailingZeros(half);
        half >>= 1u;

        var tri: u32;
        if      (bary.x > 0.5) { tri = 1u; bary = vec3<f32>(2.0*bary.x - 1.0, 2.0*bary.y,        2.0*bary.z);        }
        else if (bary.y > 0.5) { tri = 2u; bary = vec3<f32>(2.0*bary.x,        2.0*bary.y - 1.0,  2.0*bary.z);        }
        else if (bary.z > 0.5) { tri = 3u; bary = vec3<f32>(2.0*bary.x,        2.0*bary.y,        2.0*bary.z - 1.0);  }
        else                   { tri = 0u; bary = vec3<f32>(1.0 - 2.0*bary.z,  1.0 - 2.0*bary.x,  1.0 - 2.0*bary.y); }

        sub.x |= ((tri & 1u) << bit);
        sub.y |= (((tri >> 1u) & 1u) << bit);
    }

    return sub;
}

fn octahedron(octant: vec3<u32>, subdivision: vec2<u32>, count: u32) -> vec3<f32> {
    var a = vec3<f32>(select(1.0, -1.0, octant.x != 0u), 0.0, 0.0);
    var b = vec3<f32>(0.0, select(1.0, -1.0, octant.y != 0u), 0.0);
    var c = vec3<f32>(0.0, 0.0, select(1.0, -1.0, octant.z != 0u));

    var half = count >> 1u;
    while (half > 0u) {
        let bit = countTrailingZeros(half);
        let tri = ((subdivision.x >> bit) & 1u) | (((subdivision.y >> bit) & 1u) << 1u);
        half >>= 1u;

        let mab = 0.5 * (a + b);
        let mbc = 0.5 * (b + c);
        let mca = 0.5 * (c + a);

        a = select(select(mab, a,   tri == 1u), mca, tri == 3u);
        b = select(select(mbc, mab, tri == 1u), b,   tri == 2u);
        c = select(select(mca, mbc, tri == 2u), c,   tri == 3u);
    }

    return normalize(a + b + c);
}
