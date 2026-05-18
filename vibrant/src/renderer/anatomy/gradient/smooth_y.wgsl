@group(0) @binding(0) var SOURCE: texture_storage_3d<rgba8unorm, read>;
@group(0) @binding(2) var PING: texture_storage_3d<rgba8unorm, read>;
@group(0) @binding(3) var PONG: texture_storage_3d<rgba8unorm, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(SOURCE))) { return; }

    let c0 = unpack_rgb(textureLoad(PING, voxel));
    let p1 = unpack_rgb(textureLoad(PING, voxel + vec3<u32>(0, 1, 0)));
    let m1 = unpack_rgb(textureLoad(PING, voxel - vec3<u32>(0, 1, 0)));
    let p2 = unpack_rgb(textureLoad(PING, voxel + vec3<u32>(0, 2, 0)));
    let m2 = unpack_rgb(textureLoad(PING, voxel - vec3<u32>(0, 2, 0)));

    let smoothing =
        KERNEL_SMOOTHING[0] * c0 +
        KERNEL_SMOOTHING[1] * (p1 + m1) +
        KERNEL_SMOOTHING[2] * (p2 + m2);

    textureStore(PONG, voxel, pack_rgb(smoothing));
}
