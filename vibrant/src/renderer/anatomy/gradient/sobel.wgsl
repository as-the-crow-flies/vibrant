@group(0) @binding(0) var SOURCE: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(1) var GRADIENT: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (any(id >= textureDimensions(SOURCE))) { return; }

    let voxel = vec3<i32>(id);

    let tri = vec3<f32>(0.25, 0.50, 0.25);
    let dif = vec3<f32>(1.00, 0.00,-1.00);

    var gradient = vec3<f32>(0.0);

    for (var x = -1; x<=1; x++) {
        for (var y = -1; y<=1; y++) {
            for (var z = -1; z<=1; z++) {
                let sample = length(textureLoad(SOURCE, voxel + vec3<i32>(x, y, z)).xyz);
                let weight = vec3<f32>(
                    dif[x+1] * tri[y+1] * tri[z+1],
                    tri[x+1] * dif[y+1] * tri[z+1],
                    tri[x+1] * tri[y+1] * dif[z+1],
                );

                gradient += sample * weight;
            }
        }
    }

    textureStore(GRADIENT, voxel, vec4<f32>(0.5 + 0.5 * gradient, length(gradient)));
}
