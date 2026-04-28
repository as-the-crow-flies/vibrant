@group(0) @binding(0) var SOURCE: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(1) var GRADIENT: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(2) var PING: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(3) var PONG: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(SOURCE))) { return; }

    let smoothing =
        KERNEL_SMOOTHING[0] *  textureLoad(PING, voxel) +
        KERNEL_SMOOTHING[1] * (textureLoad(PING, voxel + vec3<u32>(1, 0, 0)) + textureLoad(PING, voxel - vec3<u32>(1, 0, 0))) +
        KERNEL_SMOOTHING[2] * (textureLoad(PING, voxel + vec3<u32>(2, 0, 0)) + textureLoad(PING, voxel - vec3<u32>(2, 0, 0)));

    textureStore(PONG, voxel, smoothing);
}
