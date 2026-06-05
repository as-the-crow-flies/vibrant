@group(0) @binding( 0) var IRRADIANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding( 1) var RADIANCE_0: texture_3d<f32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = textureDimensions(IRRADIANCE);

    if (any(voxel >= dim)) { return; }

    let irradiance = 0.125 * (
        textureLoad(RADIANCE_0, voxel + vec3<u32>(0     , 0     , 0     ), 0) +
        textureLoad(RADIANCE_0, voxel + vec3<u32>(dim.x , 0     , 0     ), 0) +
        textureLoad(RADIANCE_0, voxel + vec3<u32>(0     , dim.y , 0     ), 0) +
        textureLoad(RADIANCE_0, voxel + vec3<u32>(0     , 0     , dim.z ), 0) +
        textureLoad(RADIANCE_0, voxel + vec3<u32>(dim.x , dim.y , 0     ), 0) +
        textureLoad(RADIANCE_0, voxel + vec3<u32>(dim.x , 0     , dim.z ), 0) +
        textureLoad(RADIANCE_0, voxel + vec3<u32>(0     , dim.y , dim.z ), 0) +
        textureLoad(RADIANCE_0, voxel + vec3<u32>(dim.x , dim.y , dim.z ), 0));

    textureStore(IRRADIANCE, voxel, pack_rgb(irradiance.rgb));
}
