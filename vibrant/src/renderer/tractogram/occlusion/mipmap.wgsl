@group(0) @binding(0) var SRC: texture_3d<f32>;
@group(0) @binding(2) var DST: texture_storage_3d<r32float, write>;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let v = 2 * voxel;

    textureStore(DST, voxel, min(min(min(min(min(min(min(
        textureLoad(SRC, v + vec3<u32>(0, 0, 0), 0),
        textureLoad(SRC, v + vec3<u32>(0, 0, 1), 0)),
        textureLoad(SRC, v + vec3<u32>(0, 1, 0), 0)),
        textureLoad(SRC, v + vec3<u32>(0, 1, 1), 0)),
        textureLoad(SRC, v + vec3<u32>(1, 0, 0), 0)),
        textureLoad(SRC, v + vec3<u32>(1, 0, 1), 0)),
        textureLoad(SRC, v + vec3<u32>(1, 1, 0), 0)),
        textureLoad(SRC, v + vec3<u32>(1, 1, 1), 0)));
}
