@group(0) @binding(0) var ABSORPTION: texture_storage_3d<r32uint, read_write>;
@group(0) @binding(1) var SCATTERING: texture_storage_3d<r32uint, read_write>;
@group(0) @binding(2) var EXTINCTION: texture_storage_3d<r32uint, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(ABSORPTION))) { return; }

    textureStore(ABSORPTION, voxel, vec4<u32>(0u));
    textureStore(SCATTERING, voxel, vec4<u32>(0u));
    textureStore(EXTINCTION, voxel, vec4<u32>(0u));
}
