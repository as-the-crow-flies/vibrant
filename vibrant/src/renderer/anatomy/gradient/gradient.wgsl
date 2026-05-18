@group(0) @binding(1) var GRADIENT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(2) var PING: texture_storage_3d<rgba8unorm, read>;
@group(0) @binding(3) var PONG: texture_storage_3d<rgba8unorm, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(GRADIENT))) { return; }

    let gradient = vec3<f32>(
        KERNEL_SMOOTHING[1] * (
            textureLoad(PING, voxel + vec3<u32>(1, 0, 0)).a -
            textureLoad(PING, voxel - vec3<u32>(1, 0, 0)).a) +
        KERNEL_SMOOTHING[2] * (
            textureLoad(PING, voxel + vec3<u32>(2, 0, 0)).a -
            textureLoad(PING, voxel - vec3<u32>(2, 0, 0)).a),
        KERNEL_SMOOTHING[1] * (
            textureLoad(PING, voxel + vec3<u32>(0, 1, 0)).a -
            textureLoad(PING, voxel - vec3<u32>(0, 1, 0)).a) +
        KERNEL_SMOOTHING[2] * (
            textureLoad(PING, voxel + vec3<u32>(0, 2, 0)).a -
            textureLoad(PING, voxel - vec3<u32>(0, 2, 0)).a),
        KERNEL_SMOOTHING[1] * (
            textureLoad(PING, voxel + vec3<u32>(0, 0, 1)).a -
            textureLoad(PING, voxel - vec3<u32>(0, 0, 1)).a) +
        KERNEL_SMOOTHING[2] * (
            textureLoad(PING, voxel + vec3<u32>(0, 0, 2)).a -
            textureLoad(PING, voxel - vec3<u32>(0, 0, 2)).a)
        );

    textureStore(GRADIENT, voxel, vec4<f32>(0.5 + 0.5 * gradient, length(gradient) + 1E-5));
}
