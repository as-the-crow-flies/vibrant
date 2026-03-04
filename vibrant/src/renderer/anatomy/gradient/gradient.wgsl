@group(0) @binding(0) var SOURCE: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(1) var GRADIENT: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(2) var PING: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(3) var PONG: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(SOURCE))) { return; }

    let gradient = vec3<f32>(
        KERNEL_SMOOTHING[1] * (
            length(textureLoad(PING, voxel + vec3<u32>(1, 0, 0)).rgb) -
            length(textureLoad(PING, voxel - vec3<u32>(1, 0, 0)).rgb)) +
        KERNEL_SMOOTHING[2] * (
            length(textureLoad(PING, voxel + vec3<u32>(2, 0, 0)).rgb) -
            length(textureLoad(PING, voxel - vec3<u32>(2, 0, 0)).rgb)),
        KERNEL_SMOOTHING[1] * (
            length(textureLoad(PING, voxel + vec3<u32>(0, 1, 0)).rgb) -
            length(textureLoad(PING, voxel - vec3<u32>(0, 1, 0)).rgb)) +
        KERNEL_SMOOTHING[2] * (
            length(textureLoad(PING, voxel + vec3<u32>(0, 2, 0)).rgb) -
            length(textureLoad(PING, voxel - vec3<u32>(0, 2, 0)).rgb)),
        KERNEL_SMOOTHING[1] * (
            length(textureLoad(PING, voxel + vec3<u32>(0, 0, 1)).rgb) -
            length(textureLoad(PING, voxel - vec3<u32>(0, 0, 1)).rgb)) +
        KERNEL_SMOOTHING[2] * (
            length(textureLoad(PING, voxel + vec3<u32>(0, 0, 2)).rgb) -
            length(textureLoad(PING, voxel - vec3<u32>(0, 0, 2)).rgb))
        );

    textureStore(GRADIENT, voxel, vec4<f32>(0.5 + 0.5 * gradient, length(gradient) + 1E-5));
}
