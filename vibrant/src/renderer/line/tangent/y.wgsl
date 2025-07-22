@group(0) @binding(0) var PING: texture_storage_3d<rgba32float, read_write>;
@group(0) @binding(1) var PONG: texture_storage_3d<rgba32float, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let one = vec3<u32>(0, 1, 0);

    let pong =
        textureLoad(PING, voxel - one) * vec4<f32>(1.0, 1.0, 1.0, 0.0) +
        textureLoad(PING, voxel      ) * vec4<f32>(2.0, 0.0, 2.0, 1.0) +
        textureLoad(PING, voxel + one) * vec4<f32>(1.0,-1.0, 1.0, 0.0);

    textureStore(PONG, voxel, pong);
}
