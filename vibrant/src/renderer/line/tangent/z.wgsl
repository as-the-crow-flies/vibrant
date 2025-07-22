@group(0) @binding(0) var PING: texture_storage_3d<rgba32float, read_write>;
@group(0) @binding(1) var PONG: texture_storage_3d<rgba32float, read_write>;

@group(1) @binding(0) var TANGENT: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let one = vec3<u32>(0, 0, 1);

    let gradient =
        textureLoad(PONG, voxel - one) * vec4<f32>(1.0, 1.0, 1.0, 0.0) +
        textureLoad(PONG, voxel      ) * vec4<f32>(2.0, 2.0, 0.0, 1.0) +
        textureLoad(PONG, voxel + one) * vec4<f32>(1.0, 1.0,-1.0, 0.0);

    let tangent = orthogonal_positive_tangent(normalize(gradient.xyz));

    textureStore(TANGENT, voxel, vec4<f32>(tangent, gradient.a));
}

fn orthogonal_positive_tangent(gradient: vec3<f32>) -> vec3<f32> {
    // Pick a seed that's not aligned with gradient (cross-safe)
    let seed = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(gradient.x) > 0.9);

    // Orthogonalize: project seed onto plane orthogonal to gradient
    let tangent = normalize(seed - gradient * dot(seed, gradient));

    // Clamp to positive orthant
    return abs(tangent);
}
