@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(0) var COLOR: texture_storage_3d<rgba8unorm, read_write>;
@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {

    let v000 = textureLoad(DENSITY, voxel, 0).x;
    // let v001 = textureLoad(DENSITY, voxel + vec3<u32>(0, 0, 1), 0).x;
    // let v010 = textureLoad(DENSITY, voxel + vec3<u32>(0, 1, 0), 0).x;
    // let v100 = textureLoad(DENSITY, voxel + vec3<u32>(1, 0, 0), 0).x;

    // let tangent = normalize(vec3<f32>(v001 - v000, v010 - v000, v100 - v000));
    // let color = vec4<f32>(tangent, v000);

    let color = vec4<f32>(vec3<f32>(1.0), v000);

    textureStore(COLOR, voxel, color);
}
