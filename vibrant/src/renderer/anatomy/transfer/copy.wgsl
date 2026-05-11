@group(0) @binding(0) var ABSORPTION_SRC: texture_storage_3d<r32uint, read>;
@group(0) @binding(1) var SCATTERING_SRC: texture_storage_3d<r32uint, read>;
@group(0) @binding(2) var EXTINCTION_SRC: texture_storage_3d<r32uint, read>;
@group(0) @binding(3) var ABSORPTION_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(4) var SCATTERING_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(5) var EXTINCTION_DST: texture_storage_3d<rgba8unorm, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(ABSORPTION_SRC))) { return; }

    textureStore(ABSORPTION_DST, voxel, unpack4x8unorm(textureLoad(ABSORPTION_SRC, voxel).x));
    textureStore(SCATTERING_DST, voxel, unpack4x8unorm(textureLoad(SCATTERING_SRC, voxel).x));
    textureStore(EXTINCTION_DST, voxel, unpack4x8unorm(textureLoad(EXTINCTION_SRC, voxel).x));
}
