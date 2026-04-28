@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var DESTINATION: texture_storage_2d<rgba16float, write>;

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let pixel = voxel.xy;

    if (any(pixel >= textureDimensions(SOURCE))) { return; }

    let N = uv_to_dir(vec2<f32>(pixel) / vec2<f32>(textureDimensions(SOURCE)));
    var prefiltered = vec4<f32>(0.0);

    let sample_count = 8192u;
    for (var i=0u; i<sample_count; i++) {
        let Xi = hammersley(i, sample_count);

        let H = importance_sample_ggx(Xi, N, 0.1);
        let L = normalize(2.0 * dot(N, H) * H - N);

        let NdotL = max(dot(N, L), 0.0);
        if(NdotL > 0.0) {
            let sample = textureSampleLevel(SOURCE, SAMPLER, equirectangular(L, 0.0), 0.0);

            prefiltered += vec4<f32>(sample.rgb, 1.0) * NdotL;
        }
    }

    textureStore(DESTINATION, pixel, prefiltered / prefiltered.a);
}

fn uv_to_dir(uv: vec2<f32>) -> vec3<f32> {
    let theta = (0.5 - uv.x) * 2.0 * PI;
    let phi = uv.y * PI;

    let sin_phi = sin(phi);

    return vec3<f32>(
        cos(theta) * sin_phi,
        cos(phi),
        sin(theta) * sin_phi
    );
}
