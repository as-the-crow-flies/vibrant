@group(0) @binding(2) var EXTINCTION: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(3) var GRADIENT: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(EXTINCTION))) { return; }

    let p0 = sample(voxel);

    for (var axis = 0u; axis < 3u; axis++) {
        let offset = axis * vec3<u32>(0, 0, textureDimensions(EXTINCTION).z);
        let direction = 2 * vec3<u32>(u32(axis == 0), u32(axis == 1), u32(axis == 2));
        let gradient = p0 - 0.5 * (sample(voxel - direction) + sample(voxel + direction));
        textureStore(GRADIENT, offset + voxel, vec4<f32>(gradient, 0.0));
    }
}

fn sample(voxel: vec3<u32>) -> vec3<f32> {
    return textureLoad(EXTINCTION, voxel).rgb;
}
