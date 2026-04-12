struct Material {
    absorption: vec4<f32>,
    scattering: vec4<f32>,
};

@group(0) @binding(0) var FRACTION: texture_3d<f32>;
@group(0) @binding(3) var<uniform> MATERIAL: Material;

@group(1) @binding(0) var ABSORPTION: texture_storage_3d<rgba8unorm, read_write>;
@group(1) @binding(1) var SCATTERING: texture_storage_3d<rgba8unorm, read_write>;
@group(1) @binding(2) var EXTINCTION: texture_storage_3d<rgba8unorm, read_write>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let crop_min = 0.5 + vec3<f32>(
        ENVIRONMENT.settings.crop_x_start,
        ENVIRONMENT.settings.crop_y_start,
        ENVIRONMENT.settings.crop_z_start,
    );

    let crop_min_u32 = vec3<u32>(crop_min * vec3<f32>(textureDimensions(ABSORPTION)));

    let crop_max = 0.5 + vec3<f32>(
        ENVIRONMENT.settings.crop_x_end,
        ENVIRONMENT.settings.crop_y_end,
        ENVIRONMENT.settings.crop_z_end,
    );

    let crop_max_u32 = vec3<u32>(crop_max * vec3<f32>(textureDimensions(ABSORPTION)));

    if (any(voxel < crop_min_u32) | any(voxel > crop_max_u32) |
        any(voxel >= textureDimensions(ABSORPTION)))
        { return; }

    let fraction = textureLoad(FRACTION, voxel, 0).x;
    let absorption = fraction * MATERIAL.absorption;
    let scattering = fraction * MATERIAL.scattering;

    textureStore(ABSORPTION, voxel, textureLoad(ABSORPTION, voxel) + absorption);
    textureStore(SCATTERING, voxel, textureLoad(SCATTERING, voxel) + scattering);
    textureStore(EXTINCTION, voxel, textureLoad(EXTINCTION, voxel) + absorption + scattering);
}
