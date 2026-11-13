// Isotropic GGX reflection in a local frame with the normal along +z.
use math;

fn D_GGX(n_dot_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

// Bounded VNDF sampling (Eto & Tokuyoshi 2023) on spherical caps (Dupuy &
// Benyoub 2023): the cap is cut to the visible normals whose reflection stays
// above the horizon. Returns the reflected direction.
fn sample_bounded_vndf(wi: vec3<f32>, alpha: f32, u: vec2<f32>) -> vec3<f32> {
    let wi_std = normalize(vec3<f32>(wi.xy * alpha, wi.z));

    let b = select(wi_std.z, bounded_k(wi, alpha) * wi_std.z, wi.z > 0.0);

    let phi = 2.0 * PI * u.x;
    let z = (1.0 - u.y) * (1.0 + b) - b;
    let sin_theta = sqrt(clamp(1.0 - z * z, 0.0, 1.0));
    let c_std = vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), z);

    let wm_std = c_std + wi_std;
    let wm = normalize(vec3<f32>(wm_std.xy * alpha, wm_std.z));

    return reflect(-wi, wm);
}

// Solid-angle pdf of `wo` under `sample_bounded_vndf`.
fn bounded_vndf_pdf(wi: vec3<f32>, wo: vec3<f32>, alpha: f32) -> f32 {
    let wm = normalize(wi + wo);
    let ndf = D_GGX(wm.z, alpha);

    let ai = alpha * wi.xy;
    let len2 = dot(ai, ai);
    let t = sqrt(len2 + wi.z * wi.z);

    if (wi.z >= 0.0) {
        return ndf / (2.0 * (bounded_k(wi, alpha) * wi.z + t));
    }

    return ndf * (t - wi.z) / (2.0 * len2);
}

fn bounded_k(wi: vec3<f32>, alpha: f32) -> f32 {
    let a = clamp(alpha, 0.0, 1.0);
    let s = 1.0 + length(wi.xy);
    let a2 = a * a;
    let s2 = s * s;
    return (1.0 - a2) * s2 / (s2 + a2 * wi.z * wi.z);
}

// Duff et al. 2017, "Building an Orthonormal Basis, Revisited": a frame with
// `n` as its third column.
fn orthonormal_frame(n: vec3<f32>) -> mat3x3<f32> {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;

    return mat3x3<f32>(
        vec3<f32>(1.0 + s * n.x * n.x * a, s * b, -s * n.x),
        vec3<f32>(b, s + n.y * n.y * a, -n.y),
        n,
    );
}

fn uniform_sphere(u: vec2<f32>) -> vec3<f32> {
    let z = 1.0 - 2.0 * u.x;
    let r = sqrt(max(1.0 - z * z, 0.0));
    let phi = 2.0 * PI * u.y;
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}
