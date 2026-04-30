struct Material {
    absorption: vec4<f32>,
    scattering: vec4<f32>,
    min: f32,
    max: f32,
    inverted: u32,
    masked: u32,
    use_colormap: u32,
    colormap: u32,
};

struct MaskSettings {
    visible: u32,
    invert: u32,
    binary: u32,
    offset: f32,
    width: f32
};

@group(0) @binding(0) var FRACTION: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(0) @binding(3) var<uniform> MATERIAL: Material;
@group(0) @binding(4) var COLORMAP: texture_2d<f32>;

@group(1) @binding(0) var MASK: texture_3d<f32>;
@group(1) @binding(1) var<uniform> MASK_SETTINGS: MaskSettings;

@group(2) @binding(0) var ABSORPTION: texture_storage_3d<rgba8unorm, read_write>;
@group(2) @binding(1) var SCATTERING: texture_storage_3d<rgba8unorm, read_write>;
@group(2) @binding(2) var EXTINCTION: texture_storage_3d<rgba8unorm, read_write>;

@group(3) @binding(0) var<uniform> CROP: CropSettings;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = vec3<f32>(textureDimensions(ABSORPTION));

    let crop_min = 0.5 + CROP.min.xyz;
    let crop_max = 0.5 + CROP.max.xyz;

    let crop_min_u32 = vec3<u32>(crop_min * dim);
    let crop_max_u32 = vec3<u32>(crop_max * dim);

    if (any(voxel < crop_min_u32) | any(voxel > crop_max_u32) |
        any(voxel >= textureDimensions(ABSORPTION)))
        { return; }

    let crop_normal = normal_from_spherical(CROP.spherical.x, CROP.spherical.y);
    let voxel_f32 = vec3<f32>(voxel) / dim - 0.5;
    let voxel_distance = dot(crop_normal, voxel_f32);

    let voxel_distance_transform = smoothstep(
        -CROP.spherical.w, CROP.spherical.w,
        0.5 - CROP.spherical.z - voxel_distance);

    let uv = vec3<f32>(voxel) / dim;
    var fraction = textureSampleLevel(FRACTION, SAMPLER, uv, 0.0).x;
    if (bool(MATERIAL.inverted) && fraction != 0.0) { fraction = 1.0 - fraction; }

    var color = vec3<f32>(fraction);

    if (bool(MATERIAL.use_colormap) && all(color != vec3<f32>(0.0))) {
        color = textureLoad(COLORMAP, vec2<u32>(u32(fraction * 255.0), MATERIAL.colormap), 0).rgb;
    }

    color = (color - MATERIAL.min) / (MATERIAL.max - MATERIAL.min);

    color *= get_mask(uv) * voxel_distance_transform;
    color = saturate(color);

    var absorption = MATERIAL.absorption.rgb * MATERIAL.absorption.a;
    var scattering = MATERIAL.scattering.rgb * MATERIAL.scattering.a;

    if (bool(MATERIAL.use_colormap)) {
        absorption *= invert_hue_approx(color);
        scattering *= color;
    } else {
        absorption *= color;
        scattering *= color;
    }

    textureStore(ABSORPTION, voxel, textureLoad(ABSORPTION, voxel) + vec4<f32>(absorption, 1.0));
    textureStore(SCATTERING, voxel, textureLoad(SCATTERING, voxel) + vec4<f32>(scattering, 1.0));
    textureStore(EXTINCTION, voxel, textureLoad(EXTINCTION, voxel) + vec4<f32>(absorption + scattering, 1.0));
}

fn get_mask(uv: vec3<f32>) -> f32 {
    var mask = textureSampleLevel(MASK, SAMPLER, uv, 0.0).x;

    if (bool(MASK_SETTINGS.binary)) {
        mask = select(mask, 1.0 - mask, bool(MASK_SETTINGS.invert));
        mask = saturate(mask + MASK_SETTINGS.offset);
    } else {
        mask = select(1.0, -1.0, bool(MASK_SETTINGS.invert)) * mask;
        mask = smoothstep(MASK_SETTINGS.offset - MASK_SETTINGS.width, MASK_SETTINGS.offset + MASK_SETTINGS.width, mask);
    }

    mask = select(1.0, mask, bool(MASK_SETTINGS.visible));
    mask = select(1.0, mask, bool(MATERIAL.masked));

    return mask;
}

fn normal_from_spherical(phi: f32, theta: f32) -> vec3<f32> {
    let sin_theta = sin(theta);
    return vec3<f32>(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos(theta)
    );
}

fn invert_hue_approx(color: vec3<f32>) -> vec3<f32> {
    let luma = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    return vec3<f32>(2.0 * luma) - color;
}
