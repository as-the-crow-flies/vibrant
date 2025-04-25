@group(0) @binding(0) var OCCLUSION: texture_storage_3d<r8unorm, read_write>;
@group(0) @binding(3) var<uniform> OCCLUSION_TO_PROJECTION: mat4x4<f32>;

@group(1) @binding(0) var THRESHOLD: texture_storage_2d<r8unorm, read_write>;
@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel = id.xy;
    let max_depth = textureDimensions(OCCLUSION).z;

    var optical_depth = 0.0;
    var threshold = 1.0;

    for (var froxel = vec3<u32>(pixel, 0); froxel.z < max_depth; froxel.z++) {
        optical_depth += textureLoad(OCCLUSION, froxel).x;
        textureStore(OCCLUSION, froxel, vec4<f32>(optical_depth));

        if (threshold == 1.0 && optical_depth > ENVIRONMENT.settings.culling_threshold) {
            threshold = f32(froxel.z + 1u) / f32(max_depth);
        }
    }

    textureStore(THRESHOLD, pixel, vec4<f32>(threshold));
}
