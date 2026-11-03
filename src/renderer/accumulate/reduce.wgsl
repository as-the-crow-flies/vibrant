// Convergence metric: one workgroup strides over every texel of the current
// accumulated mean (`NEXT`) and the previous mean (`PREV`), summing the
// per-pixel luminance residual `|luma(next) - luma(prev)|` and the luminance
// itself. The CPU reads back `residual / luma` - the mean relative per-pixel
// change contributed by the last sample - and treats the image as converged
// once it drops below `Settings::noise_threshold`. The ratio is scale
// invariant and falls off roughly as 1/sample.

@group(0) @binding(0) var PREV: texture_2d<f32>;
@group(0) @binding(1) var PREV_SAMPLER: sampler;

@group(1) @binding(0) var NEXT: texture_2d<f32>;
@group(1) @binding(1) var NEXT_SAMPLER: sampler;

struct Metric {
    residual: f32,
    luma: f32,
    count: f32,
    _pad: f32,
};
@group(2) @binding(0) var<storage, read_write> METRIC: Metric;

const THREADS: u32 = 256u;

var<workgroup> sh_residual: array<f32, THREADS>;
var<workgroup> sh_luma: array<f32, THREADS>;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@compute @workgroup_size(THREADS)
fn reduce(@builtin(local_invocation_id) lid: vec3<u32>) {
    let dims = textureDimensions(NEXT);
    let total = dims.x * dims.y;

    var residual = 0.0;
    var luma = 0.0;

    var i = lid.x;
    loop {
        if (i >= total) { break; }

        let coord = vec2<u32>(i % dims.x, i / dims.x);
        let n = luminance(textureLoad(NEXT, coord, 0).rgb);
        let p = luminance(textureLoad(PREV, coord, 0).rgb);

        residual += abs(n - p);
        luma += n;

        i += THREADS;
    }

    sh_residual[lid.x] = residual;
    sh_luma[lid.x] = luma;
    workgroupBarrier();

    var stride = THREADS >> 1u;
    loop {
        if (stride == 0u) { break; }
        if (lid.x < stride) {
            sh_residual[lid.x] += sh_residual[lid.x + stride];
            sh_luma[lid.x] += sh_luma[lid.x + stride];
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }

    if (lid.x == 0u) {
        METRIC.residual = sh_residual[0];
        METRIC.luma = sh_luma[0];
        METRIC.count = f32(total);
        METRIC._pad = 0.0;
    }
}
