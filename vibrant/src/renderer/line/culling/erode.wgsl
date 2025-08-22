@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(3) var ERODE: texture_storage_3d<r32float, read_write>;
@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let voxel = vec3<i32>(id);

    var density = textureLoad(DENSITY, voxel + vec3<i32>( 0, 0, 0), 0).x;

    if (density > 0.0) {
        density = min(min(min(min(min(min(density,
            textureLoad(DENSITY, voxel + vec3<i32>( 0, 0, 1), 0).x),
            textureLoad(DENSITY, voxel + vec3<i32>( 0, 0,-1), 0).x),
            textureLoad(DENSITY, voxel + vec3<i32>( 0, 1, 0), 0).x),
            textureLoad(DENSITY, voxel + vec3<i32>( 0,-1, 0), 0).x),
            textureLoad(DENSITY, voxel + vec3<i32>( 1, 0, 0), 0).x),
            textureLoad(DENSITY, voxel + vec3<i32>(-1, 0, 0), 0).x);
    }

    textureStore(ERODE, voxel, vec4<f32>(density));
}
