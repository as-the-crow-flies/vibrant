struct Material {
    absorption: u32,
    scattering: u32,
};

@group(0) @binding(0) var SEGMENTATION: texture_3d<u32>;
@group(0) @binding(3) var<storage, read> MATERIAL: array<Material>;

@group(1) @binding(0) var ABSORPTION_TRANSMISSION: texture_storage_3d<rgba8unorm, write>;
@group(1) @binding(1) var SCATTERING_ROUGHNESS: texture_storage_3d<rgba8unorm, write>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(ABSORPTION_TRANSMISSION))) { return; }

    let material = MATERIAL[textureLoad(SEGMENTATION, voxel, 0).x];

    let absorption = unpack4x8unorm(material.absorption).rgb;
    let scattering = unpack4x8unorm(material.scattering).rgb;

    let transmission = 1.0;
    let roughness = 1.0;

    textureStore(ABSORPTION_TRANSMISSION, voxel, vec4<f32>(absorption, transmission));
    textureStore(SCATTERING_ROUGHNESS, voxel, vec4<f32>(scattering, roughness));
}
