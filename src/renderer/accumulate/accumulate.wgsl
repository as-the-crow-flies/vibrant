// Progressive accumulation: fold this frame's freshly traced sample (`COLOR`)
// into the running mean held in `PREV`, writing the new mean to the bound
// target. Stores a running *mean* (not a sum) via a lerp, so an rgba16float
// target stays precise across hundreds of samples.
//
// `ACC.sample` is the number of samples already accumulated (0 for the first).

@group(0) @binding(0) var COLOR: texture_2d<f32>;
@group(0) @binding(1) var COLOR_SAMPLER: sampler;

@group(1) @binding(0) var PREV: texture_2d<f32>;
@group(1) @binding(1) var PREV_SAMPLER: sampler;

struct Accumulate {
    sample: u32,
};
@group(2) @binding(0) var<uniform> ACC: Accumulate;

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
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let coord = vec2<u32>(position.xy);
    let color = textureLoad(COLOR, coord, 0);

    if (ACC.sample == 0u) {
        return color;
    }

    let prev = textureLoad(PREV, coord, 0);
    let weight = 1.0 / f32(ACC.sample + 1u);
    return mix(prev, color, weight);
}
