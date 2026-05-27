@group(0) @binding(1) var GRADIENT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(2) var PING: texture_storage_3d<rgba8unorm, read>;
@group(0) @binding(3) var PONG: texture_storage_3d<rgba8unorm, write>;

fn load(vi: vec3<i32>) -> f32 {
    let dims = vec3<i32>(textureDimensions(PING));
    return textureLoad(PING, clamp(vi, vec3<i32>(0), dims - vec3<i32>(1))).a;
}

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(GRADIENT))) { return; }

    let vi = vec3<i32>(voxel);

    let gradient = vec3<f32>(
        KERNEL_DIFFERENCE[1] * (load(vi + vec3(1, 0, 0)) - load(vi - vec3(1, 0, 0))) +
        KERNEL_DIFFERENCE[2] * (load(vi + vec3(2, 0, 0)) - load(vi - vec3(2, 0, 0))),
        KERNEL_DIFFERENCE[1] * (load(vi + vec3(0, 1, 0)) - load(vi - vec3(0, 1, 0))) +
        KERNEL_DIFFERENCE[2] * (load(vi + vec3(0, 2, 0)) - load(vi - vec3(0, 2, 0))),
        KERNEL_DIFFERENCE[1] * (load(vi + vec3(0, 0, 1)) - load(vi - vec3(0, 0, 1))) +
        KERNEL_DIFFERENCE[2] * (load(vi + vec3(0, 0, 2)) - load(vi - vec3(0, 0, 2)))
    );

    let mag = length(gradient);
    let dir = select(vec3<f32>(0.0), gradient / mag, mag > 1e-4);
    textureStore(GRADIENT, voxel, vec4<f32>(0.5 + 0.5 * dir, 0.5 * mag));
}
