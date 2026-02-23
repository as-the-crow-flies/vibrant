struct Material {
    absorption: u32,
    scattering: u32,
};

@group(0) @binding(0) var SEGMENTATION: texture_3d<u32>;
@group(0) @binding(3) var<storage, read> MATERIAL: array<Material>;

@group(1) @binding(0) var ABSORPTION: texture_storage_3d<rgba8unorm, write>;
@group(1) @binding(1) var SCATTERING: texture_storage_3d<rgba8unorm, write>;
@group(1) @binding(2) var EXTINCTION: texture_storage_3d<rgba8unorm, write>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(ABSORPTION))) { return; }

    let material_id = textureLoad(SEGMENTATION, voxel, 0).x;
    let material = MATERIAL[material_id];

    let absorption = unpack4x8unorm(material.absorption).rgb;
    let scattering = unpack4x8unorm(material.scattering).rgb;
    let extinction = absorption + scattering;

    textureStore(ABSORPTION, voxel, vec4<f32>(absorption, 0.0));
    textureStore(SCATTERING, voxel, vec4<f32>(scattering, 0.0));
    textureStore(EXTINCTION, voxel, vec4<f32>(extinction, 0.0));
}
