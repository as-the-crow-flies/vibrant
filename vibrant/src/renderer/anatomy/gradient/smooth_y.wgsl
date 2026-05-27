@group(0) @binding(0) var SOURCE: texture_storage_3d<rgba8unorm, read>;
@group(0) @binding(2) var PING: texture_storage_3d<rgba8unorm, read>;
@group(0) @binding(3) var PONG: texture_storage_3d<rgba8unorm, write>;

fn load(vi: vec3<i32>) -> vec3<f32> {
    let dims = vec3<i32>(textureDimensions(PING));
    return unpack_rgb(textureLoad(PING, clamp(vi, vec3<i32>(0), dims - vec3<i32>(1))));
}

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(SOURCE))) { return; }

    let vi = vec3<i32>(voxel);
    let c0 = load(vi);
    let p1 = load(vi + vec3(0, 1, 0));
    let m1 = load(vi - vec3(0, 1, 0));
    let p2 = load(vi + vec3(0, 2, 0));
    let m2 = load(vi - vec3(0, 2, 0));

    let smoothing =
        KERNEL_SMOOTHING[0] * c0 +
        KERNEL_SMOOTHING[1] * (p1 + m1) +
        KERNEL_SMOOTHING[2] * (p2 + m2);

    textureStore(PONG, voxel, pack_rgb(smoothing));
}
