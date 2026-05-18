@group(0) @binding(0) var SOURCE: texture_storage_3d<rgba8unorm, read>;
@group(0) @binding(2) var PING: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(3) var PONG: texture_storage_3d<rgba8unorm, read>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(SOURCE))) { return; }

    let c0 = unpack_rgb(textureLoad(PONG, voxel));
    let p1 = unpack_rgb(textureLoad(PONG, voxel + vec3<u32>(0, 0, 1)));
    let m1 = unpack_rgb(textureLoad(PONG, voxel - vec3<u32>(0, 0, 1)));
    let p2 = unpack_rgb(textureLoad(PONG, voxel + vec3<u32>(0, 0, 2)));
    let m2 = unpack_rgb(textureLoad(PONG, voxel - vec3<u32>(0, 0, 2)));

    let smoothing =
        KERNEL_SMOOTHING[0] * c0 +
        KERNEL_SMOOTHING[1] * (p1 + m1) +
        KERNEL_SMOOTHING[2] * (p2 + m2);

    textureStore(PING, voxel, pack_rgb(smoothing));
}
