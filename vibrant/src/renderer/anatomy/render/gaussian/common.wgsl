const VMM_SIZE   : u32 = #VMM_SIZEu;

fn vmf(v: vec4<f32>, omega: mat4x3<f32>) -> vec4<f32> {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let k  = (3.0 * u2 - 1.0) / (u2 * u - u);          // Banerjee, reparameterized
    let kl = k * LOG2_E;

    let e = exp2(fma(vec4<f32>(kl * q), transpose(omega) * v.xyz, vec4<f32>(-kl))); // e^{κ(μ·ωᵢ − 1)}
    return INV_TWO_PI * k * e / (1.0 - exp2(-2.0 * kl));
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}
