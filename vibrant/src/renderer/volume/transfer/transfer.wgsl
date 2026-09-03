struct MaterialNode {
    absorption: vec3<f32>,
    position: f32,
    scattering: vec3<f32>,
    ior: f32,
}

struct MaterialSample {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    ior: f32,
}

struct Material {
    inverted: u32,
    masked: u32,
    use_colormap: u32,
    colormap: u32,
    // Scales absorption + scattering after the transfer function. `_pad` keeps
    // `nodes` on a 16-byte boundary (see VolumeFractionSettingsBuffer).
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    nodes: array<MaterialNode, 8>,
}

struct MaskSettings {
    visible: u32,
    invert: u32,
    binary: u32,
    offset: f32,
    width: f32
};

@group(0) @binding(0) var ABSORPTION: texture_storage_3d<r32uint, read_write>;
@group(0) @binding(1) var SCATTERING: texture_storage_3d<r32uint, read_write>;
@group(0) @binding(2) var PROPERTIES: texture_storage_3d<r32uint, read_write>;

@group(1) @binding(0) var FRACTION: texture_3d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;
@group(1) @binding(3) var<uniform> MATERIAL: Material;
@group(1) @binding(4) var COLORMAP: texture_2d<f32>;

@group(2) @binding(0) var MASK: texture_3d<f32>;
@group(2) @binding(1) var<uniform> MASK_SETTINGS: MaskSettings;

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

    // `+ w` biases the smoothstep band fully past the +face at z == 0, so the
    // default ("off") spherical crop doesn't fade out the volume's outer voxels.
    let voxel_distance_transform = smoothstep(
        -CROP.spherical.w, CROP.spherical.w,
        0.5 - CROP.spherical.z + CROP.spherical.w - voxel_distance);

    let uv = vec3<f32>(voxel) / dim;
    var fraction = textureSampleLevel(FRACTION, SAMPLER, uv, 0.0).x;
    if (bool(MATERIAL.inverted) && fraction != 0.0) { fraction = 1.0 - fraction; }

    // Absorption/scattering for this voxel's intensity, interpolated across the
    // material transfer function nodes (sorted by position, unused trailing slots
    // zeroed with ior == 0.0). This replaces the old fixed per-volume tint.
    let material = sample_material(fraction);

    var absorption = material.absorption;
    var scattering = material.scattering;
    var properties = vec3<f32>(material.ior, 1.0, 0.0);

    // Recolor by the colormap, same asymmetric hue trick as before (absorption
    // takes the inverted hue so it doesn't cancel out the scattered color).
    // Note: intensity is already baked into `material` via the node lookup above,
    // so this only retints it - it must not also scale by fraction/color again.
    if (bool(MATERIAL.use_colormap) && fraction != 0.0) {
        let tint = textureLoad(COLORMAP, vec2<u32>(u32(fraction * 255.0), MATERIAL.colormap), 0).rgb;
        absorption *= invert_hue_approx(tint);
        scattering *= tint;
    }

    // Volume opacity: scales this fraction's contribution to the medium after
    // the transfer function (leaves refraction / ior in `properties` alone).
    let opacity = saturate(MATERIAL.opacity);
    absorption *= opacity;
    scattering *= opacity;

    let attenuation = get_mask(uv) * voxel_distance_transform;
    absorption *= attenuation;
    scattering *= attenuation;
    properties *= attenuation;

    absorption = unpack_rgb(unpack4x8unorm(textureLoad(ABSORPTION, voxel).x)) + absorption;
    scattering = unpack_rgb(unpack4x8unorm(textureLoad(SCATTERING, voxel).x)) + scattering;
    properties = unpack_rgb(unpack4x8unorm(textureLoad(PROPERTIES, voxel).x)) + properties;

    textureStore(ABSORPTION, voxel, vec4<u32>(pack4x8unorm(pack_rgb(absorption))));
    textureStore(SCATTERING, voxel, vec4<u32>(pack4x8unorm(pack_rgb(scattering))));
    textureStore(PROPERTIES, voxel, vec4<u32>(pack4x8unorm(pack_rgb(properties))));
}

// Piecewise-linear lookup across MATERIAL.nodes at transfer-function position `t`,
// mirroring TransferFunctionEditor::gradient() on the Rust side.
fn sample_material(t: f32) -> MaterialSample {
    var count = 0u;

    for (var i = 0u; i < 8u; i = i + 1u) {
        if (MATERIAL.nodes[i].ior <= 0.0) { break; }
        count = count + 1u;
    }

    if (count == 0u) {
        return MaterialSample(vec3<f32>(0.0), vec3<f32>(0.0), 0.0);
    }

    if (count == 1u) {
        return MaterialSample(MATERIAL.nodes[0].absorption, MATERIAL.nodes[0].scattering, MATERIAL.nodes[0].ior);
    }

    var index = count;

    for (var i = 0u; i < count; i = i + 1u) {
        if (MATERIAL.nodes[i].position >= t) {
            index = i;
            break;
        }
    }

    if (index == 0u) {
        return MaterialSample(MATERIAL.nodes[0].absorption, MATERIAL.nodes[0].scattering, MATERIAL.nodes[0].ior);
    }

    if (index == count) {
        let node = MATERIAL.nodes[count - 1u];
        return MaterialSample(node.absorption, node.scattering, node.ior);
    }

    let a = MATERIAL.nodes[index - 1u];
    let b = MATERIAL.nodes[index];
    let span = max(b.position - a.position, 0.00001);
    let factor = saturate((t - a.position) / span);

    return MaterialSample(
        mix(a.absorption, b.absorption, factor),
        mix(a.scattering, b.scattering, factor),
        mix(a.ior, b.ior, factor)
    );
}

fn get_mask(uv: vec3<f32>) -> f32 {
    var mask = textureSampleLevel(MASK, SAMPLER, uv, 0.0).x;

    if (bool(MASK_SETTINGS.binary)) {
        mask = select(mask, 1.0 - mask, bool(MASK_SETTINGS.invert));
        mask = saturate(mask + MASK_SETTINGS.offset);
    } else {
        let offset = 0.1 * MASK_SETTINGS.offset;
        let width = 0.01 * MASK_SETTINGS.width;

        mask = select(1.0, -1.0, bool(MASK_SETTINGS.invert)) * mask;
        mask = smoothstep(offset - width, offset + width, mask);
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
