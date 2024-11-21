@group(0) @binding(0) var SRC: texture_3d<f32>;
@group(1) @binding(0) var DST: texture_storage_3d<r32float, write>;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn compute(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let v = vec3<i32>(voxel);

    var minimum = 1.0;

    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                minimum = min(minimum, textureLoad(SRC, v + vec3<i32>(x, y, z), 0).x);
            }
        }
    }

    textureStore(DST, voxel, vec4<f32>(vec3<f32>(minimum), 1.0));
}
