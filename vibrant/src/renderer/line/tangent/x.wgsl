@group(0) @binding(0) var PING: texture_storage_3d<rgba32float, read_write>;
@group(0) @binding(1) var PONG: texture_storage_3d<rgba32float, read_write>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let one = vec3<u32>(1, 0, 0);

    let x = vec3<f32>(
        textureLoad(DENSITY, voxel - one, 0).x,
        textureLoad(DENSITY, voxel      , 0).x,
        textureLoad(DENSITY, voxel + one, 0).x,
    );

    let x_smoothing  = dot(x, vec3<f32>(1.0, 2.0, 1.0));
    let x_difference = dot(x, vec3<f32>(1.0, 0.0,-1.0));

    textureStore(PING, voxel, vec4<f32>(x_difference, x_smoothing, x_smoothing, x[1]));
}
