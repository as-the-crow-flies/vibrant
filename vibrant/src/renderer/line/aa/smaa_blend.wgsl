// SMAA Pass 2: Blend weight calculation.

// computes blending weights that indicate how much 
// neighboring pixels should contribute during final blending.

@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var EDGES: texture_2d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;


@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0
    );
}

// estimate blend weights from distances to edge endpoints.
fn area(dist1: f32, dist2: f32) -> vec2<f32> {
    let total = dist1 + dist2;
    if (total < 1.0) {
        return vec2<f32>(0.0);
    }

    let t = dist1 / total;
    let strength = 2.0 * (0.5 - abs(t - 0.5));
    // asymmetric weights
    return vec2<f32>(t, 1.0 - t) * strength;
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(ENVIRONMENT.surface);
    let uv = pixel.xy / resolution;
    let texel = 1.0 / resolution;
    let max_steps = i32(ENVIRONMENT.settings.smaa_max_search_steps);

    let edges = textureSample(EDGES, SAMPLER, uv).rg;

    // if no edges
    if (edges.x == 0.0 && edges.y == 0.0) {
        return vec4<f32>(0.0);
    }

    var weights = vec4<f32>(0.0);

    // vertical edge processing
    // Blend LEFT and RIGHT (weights.b/a) to smooth the staircase.
    if (edges.x > 0.0) {    // edge runs vertically
        var up_dist: f32 = 0.0;
        var down_dist: f32 = 0.0;

        // search up and down along the edge to find its length
        for (var i: i32 = 1; i <= max_steps; i++) { // up
            let sample_uv = uv + vec2<f32>(0.0, f32(-i) * texel.y);
            let e = textureSample(EDGES, SAMPLER, sample_uv).r;
            if (e < 0.5) { break; }
            up_dist = f32(i);
        }

        for (var i: i32 = 1; i <= max_steps; i++) { // down
            let sample_uv = uv + vec2<f32>(0.0, f32(i) * texel.y);
            let e = textureSample(EDGES, SAMPLER, sample_uv).r;
            if (e < 0.5) { break; }
            down_dist = f32(i);
        }

        // Compute asymmetric blend weights for left/right blending
        let a = area(up_dist, down_dist);
        weights.b = a.x;    // weight for left neighbor
        weights.a = a.y;    // right neighbor
    }

    // horizontal edge processing
    if (edges.y > 0.0) {
        var left_dist: f32 = 0.0;
        var right_dist: f32 = 0.0;

        for (var i: i32 = 1; i <= max_steps; i++) { // left
            let sample_uv = uv + vec2<f32>(f32(-i) * texel.x, 0.0);
            let e = textureSample(EDGES, SAMPLER, sample_uv).g;
            if (e < 0.5) { break; }
            left_dist = f32(i);
        }

        for (var i: i32 = 1; i <= max_steps; i++) { //right
            let sample_uv = uv + vec2<f32>(f32(i) * texel.x, 0.0);
            let e = textureSample(EDGES, SAMPLER, sample_uv).g;
            if (e < 0.5) { break; }
            right_dist = f32(i);
        }

        let a = area(left_dist, right_dist);
        weights.r = a.x;    // top neighbor
        weights.g = a.y;    // bottom neighbor
    }

    return weights;
}
