@group(0) @binding(0) var DENSITY: texture_storage_3d<r32float, read_write>;

@group(1) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(1) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    textureStore(DENSITY, voxel, vec4<f32>(0.0));
    textureStore(START, voxel, vec4<u32>(0));
    textureStore(END, voxel, vec4<u32>(0));
}
