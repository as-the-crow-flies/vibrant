@group(0) @binding(0) var HDRI: texture_2d<f32>;
@group(0) @binding(1) var HDRI_SAMPLER: sampler;
@group(0) @binding(2) var IMPORTANCE: texture_storage_2d<rgba32float, write>;

// Resamples the equirect HDRI onto Clarberg's equal-area octahedral map as (Y·ω, Y).
@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    let size = textureDimensions(IMPORTANCE);
    if (any(texel.xy >= size)) { return; }

    let uv = 2.0 * (vec2<f32>(texel.xy) + 0.5) / vec2<f32>(size) - 1.0;
    let direction = clarberg_equal_area_sphere(uv);

    let sample = equirectangular(direction.xzy, 0.0);
    let luminance = brightness(textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod(direction, size.x)).rgb);

    textureStore(IMPORTANCE, texel.xy, vec4<f32>(direction * luminance, luminance));
}

// Matches the equirect texel footprint to the equal-area texel footprint.
fn lod(direction: vec3<f32>, size: u32) -> f32 {
    let dim = vec2<f32>(textureDimensions(HDRI));
    let sin_theta = max(length(direction.xy), 1e-4);

    let texel = 4.0 * PI / f32(size * size);
    let equirect = 2.0 * PI * PI * sin_theta / (dim.x * dim.y);

    return clamp(0.5 * log2(texel / equirect), 0.0, f32(textureNumLevels(HDRI) - 1u));
}
