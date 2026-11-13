// The environment on its equal-area octahedral grid, with a luminance mip
// pyramid for hierarchical warping (Clarberg et al. 2005). Texels are equal
// area, so a texel's share of the luminance is its sampling probability over
// 4π. Roots declare `ENVIRONMENT_MAP` (rgb radiance at unit strength, a = Y).
use math;
use sphere;

fn environment_radiance(direction: vec3<f32>) -> vec3<f32> {
    return textureLoad(ENVIRONMENT_MAP, environment_texel(direction), 0).rgb * HDRI_SETTINGS.strength;
}

// Mean luminance over the sphere; 1 unless the map is black.
fn environment_mean() -> f32 {
    return textureLoad(ENVIRONMENT_MAP, vec2<u32>(0u), textureNumLevels(ENVIRONMENT_MAP) - 1u).a;
}

// Solid-angle density of `sample_environment`.
fn environment_pdf(direction: vec3<f32>) -> f32 {
    let luminance = textureLoad(ENVIRONMENT_MAP, environment_texel(direction), 0).a;
    return luminance / (4.0 * PI * environment_mean());
}

// Descends the pyramid one 2×2 block a level: a column by its luminance, then
// a row within it, rescaling `u` so the leaves stay stratified.
fn sample_environment(xi: vec2<f32>) -> vec3<f32> {
    var u = xi;
    var p = vec2<u32>(0u);

    for (var level = textureNumLevels(ENVIRONMENT_MAP) - 1u; level > 0u; level--) {
        let base = 2u * p;
        let l = level - 1u;

        let a = textureLoad(ENVIRONMENT_MAP, base, l).a;
        let b = textureLoad(ENVIRONMENT_MAP, base + vec2<u32>(1u, 0u), l).a;
        let c = textureLoad(ENVIRONMENT_MAP, base + vec2<u32>(0u, 1u), l).a;
        let d = textureLoad(ENVIRONMENT_MAP, base + vec2<u32>(1u, 1u), l).a;

        let total = a + b + c + d;
        let px = select(0.5, (a + c) / total, total > 0.0);
        let right = u.x >= px;
        u.x = select(u.x / px, (u.x - px) / (1.0 - px), right);

        let top = select(a, b, right);
        let column = top + select(c, d, right);
        let py = select(0.5, top / column, column > 0.0);
        let bottom = u.y >= py;
        u.y = select(u.y / py, (u.y - py) / (1.0 - py), bottom);

        p = base + vec2<u32>(u32(right), u32(bottom));
    }

    let size = vec2<f32>(textureDimensions(ENVIRONMENT_MAP, 0));
    let grid = (vec2<f32>(p) + clamp(u, vec2<f32>(0.0), vec2<f32>(0.99999))) / size;

    return rotation_z(-HDRI_SETTINGS.rotation * 2.0 * PI) * clarberg_equal_area_sphere(2.0 * grid - 1.0);
}

// The world `direction`'s texel; the map is baked unrotated.
fn environment_texel(direction: vec3<f32>) -> vec2<u32> {
    let local = rotation_z(HDRI_SETTINGS.rotation * 2.0 * PI) * direction;
    let size = textureDimensions(ENVIRONMENT_MAP, 0);
    let texel = vec2<u32>((clarberg_equal_area_inverse(local) + 1.0) * 0.5 * vec2<f32>(size));
    return min(texel, size - 1u);
}
