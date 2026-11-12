use uniforms;

struct MaskSettings {
    visible: u32,
    invert: u32,
    sdf: u32,
    offset: f32,
    width: f32
};

// `raw` is the sampled mask texture.
fn mask_attenuation(raw: f32, settings: MaskSettings, masked: bool) -> f32 {
    var mask = raw;

    if (bool(settings.sdf)) {
        let offset = 0.1 * settings.offset;
        let width = 0.01 * settings.width;

        mask = 1.0 - smoothstep(offset - width, offset + width, mask);
    }

    mask = select(mask, 1.0 - mask, bool(settings.invert));

    mask = select(1.0, mask, bool(settings.visible));
    mask = select(1.0, mask, masked);

    return mask;
}

// `+ w` biases the smoothstep band fully past the +face at z == 0, so the
// default ("off") spherical crop doesn't fade out the volume's outer voxels.
fn spherical_crop(crop: CropSettings, uv: vec3<f32>) -> f32 {
    let normal = normal_from_spherical(crop.spherical.x, crop.spherical.y);
    let distance = dot(normal, uv - 0.5);

    return smoothstep(
        -crop.spherical.w, crop.spherical.w,
        0.5 - crop.spherical.z + crop.spherical.w - distance);
}

fn normal_from_spherical(phi: f32, theta: f32) -> vec3<f32> {
    let sin_theta = sin(theta);
    return vec3<f32>(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos(theta)
    );
}
