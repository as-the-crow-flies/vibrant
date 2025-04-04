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
        let next = froxel + vec3<u32>(0, 0, 1);
        let step = length(transform(next) - transform(froxel));
        let absorbance = textureLoad(OCCLUSION, froxel).x;
        optical_depth += step * absorbance;
        textureStore(OCCLUSION, froxel, vec4<f32>(optical_depth));

        if (threshold == 1.0 && optical_depth > ENVIRONMENT.settings.culling_threshold) {
            let sample = OCCLUSION_TO_PROJECTION * vec4<f32>(vec3<f32>(next), 1.0);
            threshold = sample.z / sample.w;
        }
    }

    textureStore(THRESHOLD, pixel, vec4<f32>(threshold));
}

fn transform(froxel: vec3<u32>) -> vec3<f32> {
    let transform = ENVIRONMENT.camera.projection_inverse * OCCLUSION_TO_PROJECTION;
    let sample = transform * vec4<f32>(vec3<f32>(froxel), 1.0);
    return sample.xyz / sample.w * vec3<f32>(f32(ENVIRONMENT.volume));
}
