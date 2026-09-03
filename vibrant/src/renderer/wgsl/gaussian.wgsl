const EPSILON    : f32 = 1e-6;
const INV_TWO_PI : f32 = 0.15915494309189535;
const LOG2_E     : f32 = 1.4426950408889634;
const U_MIN      : f32 = 1.001;    // κ_max ≈ 1000
const U_MAX      : f32 = 100.0;    // κ_min ≈ 0.03 — indistinguishable from uniform

struct SG {
    vmf: vec4<f32>,
    phi: vec3<f32>,
}

// An Anisotropic Spherical Gaussian lobe, used below to represent a GGX NDF
// warped from the half-vector domain into the light-vector domain.
struct ASG {
    amplitude: vec3<f32>,
    basis_z: vec3<f32>,
    basis_x: vec3<f32>,
    basis_y: vec3<f32>,
    sharpness_x: f32,
    sharpness_y: f32,
}

fn evaluate_asg(asg: ASG, dir: vec3<f32>) -> vec3<f32> {
    let s_term = saturate(dot(asg.basis_z, dir));
    let lambda_term = asg.sharpness_x * dot(dir, asg.basis_x) *
                       dot(dir, asg.basis_x);
    let mu_term = asg.sharpness_y * dot(dir, asg.basis_y) *
                  dot(dir, asg.basis_y);
    return asg.amplitude * s_term * exp(-lambda_term - mu_term);
}

fn convolve_asg_sg(asg: ASG, sg_amplitude: vec3<f32>, sg_axis: vec3<f32>, sg_sharpness: f32) -> vec3<f32> {
    // The ASG paper specifes an isotropic SG as
    // exp(2 * nu * (dot(v, axis) - 1)),
    // so we must divide our SG sharpness by 2 in order
    // to get the nup parameter expected by the ASG formula
    let nu = sg_sharpness * 0.5;

    var convolve_asg: ASG;
    convolve_asg.basis_x = asg.basis_x;
    convolve_asg.basis_y = asg.basis_y;
    convolve_asg.basis_z = asg.basis_z;

    convolve_asg.sharpness_x = (nu * asg.sharpness_x) /
                                (nu + asg.sharpness_x);
    convolve_asg.sharpness_y = (nu * asg.sharpness_y) /
                                (nu + asg.sharpness_y);

    convolve_asg.amplitude = vec3<f32>(PI / sqrt((nu + asg.sharpness_x) *
                                                   (nu + asg.sharpness_y)));

    let asg_result = evaluate_asg(convolve_asg, sg_axis);
    return asg_result * sg_amplitude * asg.amplitude;
}

// https://therealmjp.github.io/posts/sg-series-part-4-specular-lighting-from-an-sg-light-source/
fn sg_specular_anisotropic(sg: SG, normal: vec3<f32>, view: vec3<f32>, roughness: f32) -> vec3<f32> {
    // Recover the light lobe's sharpness/axis from the same `v` = (Σwᵢωᵢ,
    // Σwᵢ) sufficient statistic consumed by vmf() and the other sg_*
    // helpers, instead of requiring the caller to normalize/derive them.
    let q = inverseSqrt(dot(sg.vmf.xyz, sg.vmf.xyz) + 1e-24);
    let u = clamp((sg.vmf.w + 1e-8) * q, 1.001, 100.0);
    let u2 = u * u;
    let light_sharpness = (3.0 * u2 - 1.0) / (u2 * u - u); // Banerjee, reparameterized kappa
    let light_axis = sg.vmf.xyz * q;

    let alpha = roughness * roughness;
    let alpha2 = max(alpha * alpha, 1e-8);

    // Create an SG that approximates the NDF
    let ndf_axis = normal;
    let ndf_sharpness = 2.0 / alpha2;
    let ndf_amplitude = vec3<f32>(1.0 / (PI * alpha2));

    // Apply a warpring operation that will bring the SG from
    // the half-angle domain the the the lighting domain.
    var warped_ndf: ASG;
    warped_ndf.basis_z = reflect(-view, ndf_axis);
    warped_ndf.basis_x = normalize(cross(ndf_axis, warped_ndf.basis_z));
    warped_ndf.basis_y = normalize(cross(warped_ndf.basis_z, warped_ndf.basis_x));

    // sharpness_x below scales as 1/dot_dir_o², so flooring it at the same
    // ~1e-4 epsilon used for plain divide-by-zero guards elsewhere lets the
    // warped lobe narrow (and convolve_asg_sg's closed-form amplitude, which
    // falls off as 1/sqrt(sharpness_x)) collapse toward zero far earlier than
    // any physically plausible roughness would justify - the specular term
    // goes fully black well before the view actually reaches grazing. Floor
    // it high enough to bound the lobe's sharpness (and keep the highlight
    // visible) while still letting the intended grazing-angle elongation
    // show for less extreme angles.
    let dot_dir_o = max(dot(view, ndf_axis), 0.2);

    // Second derivative of the sharpness with respect to how
    // far we are from basis Axis direction
    warped_ndf.sharpness_x = ndf_sharpness / (8.0 * dot_dir_o * dot_dir_o);
    warped_ndf.sharpness_y = ndf_sharpness / 8.0;
    warped_ndf.amplitude = ndf_amplitude;

    // Convolve the NDF with the light
    var output = convolve_asg_sg(warped_ndf, sg.phi, light_axis, light_sharpness);

    // Parameters needed for evaluating the visibility term
    let warp_dir = warped_ndf.basis_z;
    let n_dot_l = saturate(dot(normal, warp_dir));
    let n_dot_v = saturate(dot(normal, view));

    // Visibility term
    output *= GGX_V1(n_dot_l, alpha2) * GGX_V1(n_dot_v, alpha2);

    // Cosine term
    output *= n_dot_l;

    return max(output, vec3<f32>(0.0));
}

// https://therealmjp.github.io/posts/sg-series-part-4-specular-lighting-from-an-sg-light-source/
fn sg_specular(sg: SG, normal: vec3<f32>, view: vec3<f32>, roughness: f32) -> vec3<f32> {
    let q     = inverseSqrt(dot(sg.vmf.xyz, sg.vmf.xyz) + 1e-24);
    let u     = clamp((sg.vmf.w + 1e-8) * q, U_MIN, U_MAX);
    let u2    = u * u;
    let kappa = (3.0 * u2 - 1.0) / (u2 * u - u);
    let mu    = sg.vmf.xyz * q;

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

    let inner_product = sg.phi * (4.0 * n_dot_v) * ratio * scale * (1.0 - em2l3);

    let visibility = GGX_V1(n_dot_v, alpha2) * GGX_V1(n_dot_v, alpha2); // N·L = N·V at the peak

    return inner_product * visibility * n_dot_v;
}

// https://therealmjp.github.io/posts/sg-series-part-3-diffuse-lighting-from-an-sg-light-source/
fn sg_irradiance(sg: SG, normal: vec3<f32>) -> vec3<f32> {
    let q  = inverseSqrt(dot(sg.vmf.xyz, sg.vmf.xyz) + 1e-24);  // 1/|s|
    let u  = clamp((sg.vmf.w + 1e-8) * q, U_MIN, U_MAX);        // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let kappa  = (3.0 * u2 - 1.0) / (u2 * u - u);      // Banerjee, reparameterized
    let lambda = max(kappa, 1.0);

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
    let fitted = result * (2.0 * PI * sg.phi * (1.0 - em2l) / lambda);

    return mix(PI * sg.phi, fitted, clamp(kappa, 0.0, 1.0));
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
fn sg_phase(sg: SG, view: vec3<f32>, g: f32) -> vec3<f32> {
    // Recover the light lobe's sharpness/axis from `v`, as elsewhere.
    let q  = inverseSqrt(dot(sg.vmf.xyz, sg.vmf.xyz) + 1e-24);
    let u  = clamp((sg.vmf.w + 1e-8) * q, U_MIN, U_MAX);
    let u2 = u * u;
    let light_kappa = (3.0 * u2 - 1.0) / (u2 * u - u); // Banerjee, reparameterized
    let light_axis  = sg.vmf.xyz * q;

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

    // Exact SG product integral over S^2: kappa3*axis3 = kappa1*axis1 + kappa2*axis2.
    let cos_gamma = dot(light_axis, phase_axis);
    let delta     = phase_kappa * phase_kappa + 2.0 * phase_kappa * light_kappa * cos_gamma;
    let kappa3    = sqrt(max(delta + light_kappa * light_kappa, 0.0));
    let exponent  = delta / (kappa3 + light_kappa) - phase_kappa; // kappa3 - light_kappa - phase_kappa, cancellation-safe

    let em2k2 = exp2(-2.0 * phase_kappa * LOG2_E);
    let em2k3 = exp2(-2.0 * kappa3 * LOG2_E);

    // Normalized vMF phase amplitude is kappa2 / (2*pi*(1 - e^(-2*kappa2)));
    // folded into this ratio so the 2*pi cancels against the sphere
    // integral's own 2*pi*(1 - e^(-2*kappa3))/kappa3 normalization.
    let ratio = (phase_kappa / kappa3) * (1.0 - em2k3) / (1.0 - em2k2);

    return sg.phi * ratio * exp2(exponent * LOG2_E);
}

// ── Shared VMM probe lookup ───────────────────────────────────────────────
// Fetches SG lobe `k` from the octahedral radiance-cascade probe grid at the
// volume-space [0,1] coord `p`. Both the volume tracer (phase model) and the
// line tracer (surface Disney BRDF) build their per-lobe SG the same way.
// Textures + sampler are passed in so this can live in the always-included
// gaussian.wgsl without depending on any particular binding layout.
fn probe_sg(
    radiance_tex: texture_3d<f32>,
    gaussian_tex: texture_3d<f32>,
    samp: sampler,
    p: vec3<f32>,
    k: u32
) -> SG {
    let dims = vec3<f32>(textureDimensions(gaussian_tex));
    let probes = max(floor(dims / vec3<f32>(4.0, 4.0, 2.0)), vec3<f32>(1.0));
    let local = clamp(p * probes, vec3<f32>(0.5), probes - 0.5);

    let tile = vec3<f32>(vec3<u32>(k & 3u, (k >> 2u) & 3u, (k >> 4u) & 1u));
    let lobe_uv = (tile * probes + local) / dims;

    return SG(
        textureSampleLevel(gaussian_tex, samp, lobe_uv, 0.0),
        textureSampleLevel(radiance_tex, samp, lobe_uv, 0.0).rgb
    );
}

// Surface (opaque) outgoing radiance from the probe VMM: hemisphere SG diffuse
// (× albedo) + GGX-from-SG specular (× Schlick Fresnel), summed over `count`
// lobes. `diffuse_gain` / `specular_gain` are the repurposed line ambient /
// direct light sliders. Used by the tractography line tracer.
fn sample_outgoing_radiance_surface(
    radiance_tex: texture_3d<f32>,
    gaussian_tex: texture_3d<f32>,
    samp: sampler,
    p: vec3<f32>,
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

    var result = vec3<f32>(0.0);

    for (var k = 0u; k < count; k++) {
        let sg = probe_sg(radiance_tex, gaussian_tex, samp, p, k);

        result +=
            sg_irradiance(sg, normal) * albedo * diffuse_gain +
            sg_specular(sg, normal, view, roughness) * fresnel * specular_gain;
    }

    return result;
}
