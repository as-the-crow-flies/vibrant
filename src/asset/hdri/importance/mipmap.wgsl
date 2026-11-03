@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(0) @binding(1) var DESTINATION: texture_storage_2d<rgba32float, write>;

// Texels are equal-area, so a plain average is the mean over the node.
@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    if (any(texel.xy >= textureDimensions(DESTINATION))) { return; }

    let sum =
          textureLoad(SOURCE, 2u * texel.xy + vec2<u32>(0u, 0u), 0) +
          textureLoad(SOURCE, 2u * texel.xy + vec2<u32>(1u, 0u), 0) +
          textureLoad(SOURCE, 2u * texel.xy + vec2<u32>(0u, 1u), 0) +
          textureLoad(SOURCE, 2u * texel.xy + vec2<u32>(1u, 1u), 0);

    textureStore(DESTINATION, texel.xy, 0.25 * sum);
}
