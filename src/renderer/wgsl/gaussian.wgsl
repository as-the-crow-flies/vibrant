use math;
use pbr;

const EPSILON    : f32 = 1e-6;
const INV_TWO_PI : f32 = 0.15915494309189535;
const LOG2_E     : f32 = 1.4426950408889634;
const U_MIN      : f32 = 1.001;    // κ_max ≈ 1000
const U_MAX      : f32 = 100.0;    // κ_min ≈ 0.03 — indistinguishable from uniform

// A probe lobe: its vMF moments and the energy it carries, ∫ L dω.
struct SG {
    vmf: vec4<f32>,
    phi: vec3<f32>,
}

// A light lobe's axis, sharpness and peak radiance, recovered once from its
// moments and energy.
struct Lobe {
    axis: vec3<f32>,
    kappa: f32,
    phi: vec3<f32>,
}

// Peak radiance of an SG carrying `energy`: the normalized vMF's amplitude.
fn sg_amplitude(energy: vec3<f32>, kappa: f32) -> vec3<f32> {
    return energy * kappa * INV_TWO_PI / (1.0 - exp2(-2.0 * kappa * LOG2_E));
}

fn sg_lobe(sg: SG) -> Lobe {
    let q  = inverseSqrt(dot(sg.vmf.xyz, sg.vmf.xyz) + 1e-24);  // 1/|s|
    let u  = clamp((sg.vmf.w + 1e-8) * q, U_MIN, U_MAX);        // w/|s| ∈ [1, ∞)
    let u2 = u * u;
    let kappa = (3.0 * u2 - 1.0) / (u2 * u - u);               // Banerjee, reparameterized
    return Lobe(sg.vmf.xyz * q, kappa, sg_amplitude(sg.phi, kappa));
}

// A per-step SG (phase lobe or warped NDF) the light lobes are integrated against.
struct Kernel {
    axis: vec3<f32>,
    lambda: f32,
    scale: f32,
}

// Exact SG product integral over S² per unit `phi`, up to `kernel.scale`:
// λ/λ3 · (1 − e^{−2λ3}) · e^{λ3 − κ − λ}, with λ3·axis3 = κ·axis + λ·kernel.axis.
fn sg_inner_weight(lobe: Lobe, kernel: Kernel) -> f32 {
    let delta      = lobe.kappa * (lobe.kappa + 2.0 * kernel.lambda * dot(lobe.axis, kernel.axis));
    let lambda3_2  = max(delta + kernel.lambda * kernel.lambda, 1e-12);
    let lambda3_rcp = inverseSqrt(lambda3_2);
    let lambda3    = lambda3_2 * lambda3_rcp;
    let exponent   = delta / (lambda3 + kernel.lambda) - lobe.kappa; // λ3 − λ − κ, cancellation-safe

    let tail = 1.0 - exp2(-2.0 * lambda3 * LOG2_E);

    return kernel.scale * kernel.lambda * lambda3_rcp * tail * exp2(exponent * LOG2_E);
}

fn sg_inner(lobe: Lobe, kernel: Kernel) -> vec3<f32> {
    return lobe.phi * sg_inner_weight(lobe, kernel);
}

// https://therealmjp.github.io/posts/sg-series-part-4-specular-lighting-from-an-sg-light-source/
fn specular_kernel(normal: vec3<f32>, view: vec3<f32>, roughness: f32) -> Kernel {
    let n_dot_v    = max(dot(normal, view), EPSILON);
    let reflection = reflect(-view, normal); // warped NDF axis

    let alpha  = roughness * roughness;
    let alpha2 = max(alpha * alpha, 1e-8);

    let lambda_r = 0.5 / (alpha2 * n_dot_v); // = (2/α²) / (4·n_dot_v)

    let visibility = GGX_V1(n_dot_v, alpha2) * GGX_V1(n_dot_v, alpha2); // N·L = N·V at the peak

    return Kernel(reflection, lambda_r, 4.0 * n_dot_v * visibility * n_dot_v);
}

fn sg_specular(sg: SG, normal: vec3<f32>, view: vec3<f32>, roughness: f32) -> vec3<f32> {
    return sg_inner(sg_lobe(sg), specular_kernel(normal, view, roughness));
}

// https://therealmjp.github.io/posts/sg-series-part-3-diffuse-lighting-from-an-sg-light-source/
fn sg_irradiance(sg: SG, normal: vec3<f32>) -> vec3<f32> {
    let q  = inverseSqrt(dot(sg.vmf.xyz, sg.vmf.xyz) + 1e-24);  // 1/|s|
    let u  = clamp((sg.vmf.w + 1e-8) * q, U_MIN, U_MAX);        // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let kappa  = (3.0 * u2 - 1.0) / (u2 * u - u);      // Banerjee, reparameterized
    let lambda = max(kappa, 1.0);
    let phi    = sg_amplitude(sg.phi, kappa);

    let mu_dot_n = clamp(dot(sg.vmf.xyz, normal) * q, -1.0, 1.0);

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

// Represents the Henyey-Greenstein phase function as a single normalized
// vMF/SG lobe by matching mean cosines: g is by definition the mean cosine
// of the scattering angle (Henyey & Greenstein 1941), which is exactly what
// a vMF lobe's mean resultant length A_3(kappa) = coth(kappa) - 1/kappa
// measures. Setting A_3(kappa) = g and inverting with the same Banerjee
// approximation used above to recover kappa from a sufficient statistic
// (Banerjee et al. 2005, "Clustering on the Unit Hypersphere using von
// Mises-Fisher Distributions") gives a one-lobe vMF surrogate for HG(g) -
// the same substitution used by Gkioulekas et al. 2013 ("Inverse Volume
// Rendering with Material Dictionaries") to fit measured phase functions
// with combinations of HG and vMF/Fisher lobes.
//
// With the light and the phase lobe both represented as SGs, the
// in-scattered radiance toward `view` is their exact product integral over
// the full sphere - scattering has no hemisphere clamp, so unlike
// sg_irradiance this needs no fit, just the closed-form SG sphere integral
// (the same "SGInnerProduct" building block used by sg_specular's convolve
// step, here without a warp).
fn phase_kernel(view: vec3<f32>, g: f32) -> Kernel {
    // Same reparameterization applied to g in place of a reconstructed mean
    // resultant length - keeps phase_kappa bounded to the same [~0.03, ~1000]
    // range as light_kappa, so the divisions below never hit a true 0/0
    // (g = 0, isotropic scattering, would otherwise be a removable
    // singularity in the vMF normalization).
    let u_g  = clamp(1.0 / max(abs(g), 1e-4), U_MIN, U_MAX);
    let u_g2 = u_g * u_g;
    let phase_kappa = (3.0 * u_g2 - 1.0) / (u_g2 * u_g - u_g);

    // Forward scattering (g > 0) peaks when the outgoing direction of travel
    // continues along the incoming one, i.e. the light sits opposite `view`;
    // backscattering (g < 0) peaks when it sits alongside `view` instead.
    let phase_axis = select(-view, view, g < 0.0);

    // Normalized vMF phase amplitude is kappa / (2*pi*(1 - e^(-2*kappa)));
    // the 2*pi cancels against the sphere integral's own normalization.
    return Kernel(phase_axis, phase_kappa, 1.0 / (1.0 - exp2(-2.0 * phase_kappa * LOG2_E)));
}

fn sg_phase(sg: SG, view: vec3<f32>, g: f32) -> vec3<f32> {
    return sg_inner(sg_lobe(sg), phase_kernel(view, g));
}

// ── Shared VMM probe lookup ───────────────────────────────────────────────
// Both the volume tracer (phase model) and the line tracer (surface Disney
// BRDF) read lobes from the octahedral radiance-cascade probe grid this way.

// Lobe k of the probe at `p` sits at `base + tile(k) * stride`.
struct ProbeGrid {
    base: vec3<f32>,
    stride: vec3<f32>,
}

// `offset` moves the lookup, in probe cells.
fn probe_grid_at(gaussian_tex: texture_3d<f32>, p: vec3<f32>, offset: vec3<f32>) -> ProbeGrid {
    let dims = vec3<f32>(textureDimensions(gaussian_tex));
    let probes = max(floor(dims / vec3<f32>(4.0, 4.0, 2.0)), vec3<f32>(1.0));
    let local = clamp(p * probes + offset, vec3<f32>(0.5), probes - 0.5);

    return ProbeGrid(local / dims, probes / dims);
}

fn probe_grid_sg(
    radiance_tex: texture_3d<f32>,
    gaussian_tex: texture_3d<f32>,
    samp: sampler,
    grid: ProbeGrid,
    k: u32
) -> SG {
    let tile = vec3<f32>(vec3<u32>(k & 3u, (k >> 2u) & 3u, (k >> 4u) & 1u));
    let lobe_uv = grid.base + tile * grid.stride;

    return SG(
        textureSampleLevel(gaussian_tex, samp, lobe_uv, 0.0),
        textureSampleLevel(radiance_tex, samp, lobe_uv, 0.0).rgb
    );
}

// Surface outgoing radiance from the probe VMM: SG diffuse (× albedo) split
// evenly into reflection around `normal` and transmission around `-normal`
// (a Lambertian sheet, lit from both sides), + GGX-from-SG specular (× Schlick
// Fresnel), summed over `count` lobes, looked up `offset` probe cells from `p`. `diffuse_gain` / `specular_gain` are the repurposed
// line ambient / direct light sliders. Used by the tractography line tracer.
fn sample_outgoing_radiance_surface(
    radiance_tex: texture_3d<f32>,
    gaussian_tex: texture_3d<f32>,
    samp: sampler,
    p: vec3<f32>,
    offset: vec3<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    albedo: vec3<f32>,
    roughness: f32,
    f0: vec3<f32>,
    diffuse_gain: f32,
    specular_gain: f32,
    count: u32
) -> vec3<f32> {
    let light = reflect(-view, normal);
    let half = normalize(view + light);
    let fresnel = F_Schlick(max(dot(view, half), 0.0), f0);

    let diffuse = 0.5 * albedo * diffuse_gain;

    var reflected = vec3<f32>(0.0);
    var transmitted = vec3<f32>(0.0);

    let grid = probe_grid_at(gaussian_tex, p, offset);

    for (var k = 0u; k < count; k++) {
        let sg = probe_grid_sg(radiance_tex, gaussian_tex, samp, grid, k);

        reflected +=
            sg_irradiance(sg, normal) * diffuse +
            sg_specular(sg, normal, view, roughness) * fresnel * specular_gain;

        transmitted += sg_irradiance(sg, -normal) * diffuse;
    }

    return reflected + transmitted;
}
