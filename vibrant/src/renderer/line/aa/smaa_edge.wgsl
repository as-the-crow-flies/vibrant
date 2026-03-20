// SMAA Pass 1: Luma-based edge detection.

// Computes per-pixel luma and compares against 4 neighbors
// If the luma difference exceeds the threshold, the pixel is marked as an edge.
// Output: RG channels encode (horizontal_edge, vertical_edge) weights.

// Reference: Jimenez et al. "SMAA: Enhanced Subpixel Morphological Antialiasing"

@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var COLOR: texture_2d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;


/// Convert linear RGB to perceptual luma for edge detection
fn luma(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0
    );
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(ENVIRONMENT.surface);
    let uv = pixel.xy / resolution;
    // Texel size = 1 pixel in UV space.
    let texel = 1.0 / resolution;

    // Sample luma at current pixel and its 4 neighbors.
    let center = luma(textureSample(COLOR, SAMPLER, uv).rgb);
    let left   = luma(textureSample(COLOR, SAMPLER, uv + vec2<f32>(-texel.x, 0.0)).rgb);
    let top    = luma(textureSample(COLOR, SAMPLER, uv + vec2<f32>(0.0, -texel.y)).rgb);
    let right  = luma(textureSample(COLOR, SAMPLER, uv + vec2<f32>( texel.x, 0.0)).rgb);
    let bottom = luma(textureSample(COLOR, SAMPLER, uv + vec2<f32>(0.0,  texel.y)).rgb);

    // Compute absolute luma deltas against neighbors.
    let delta = abs(vec4<f32>(center) - vec4<f32>(left, top, right, bottom));

    // An edge exists if any neighbor delta exceeds the threshold.
    let threshold = ENVIRONMENT.settings.smaa_threshold;
    let edges = step(vec2<f32>(threshold), vec2<f32>(max(delta.x, delta.z), max(delta.y, delta.w)));

    // Discard pixels with no detected edges (saves blend weight computation).
    if (edges.x == 0.0 && edges.y == 0.0) {
        discard;
    }

    return vec4<f32>(edges, 0.0, 1.0);
}
