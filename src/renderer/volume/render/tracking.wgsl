// Tracking through the classified medium over its majorant grid. Roots declare
// `LINE_EXTINCTION` and `LINE_SAMPLER`.
use classify;
use noise;

// Guards against endless loops on NaN coordinates.
const TRACKING_MAX: u32 = 4096u;

// Below this throughput ratio tracking plays Russian roulette.
const ROULETTE: f32 = 0.1;

// A walk through grid cells along the ray (Amanatides & Woo 1987).
struct Walk {
    cell: vec3<u32>,
    step: vec3<i32>,
    // `t` where the ray crosses into the next cell, per axis.
    next: vec3<f32>,
    // `t` per cell, per axis.
    delta: vec3<f32>,
}

// A walk through cells of `voxels`³ voxels, `cells` per axis.
fn begin_walk(origin: vec3<f32>, direction: vec3<f32>, t: f32, voxels: u32, cells: vec3<u32>) -> Walk {
    let scale = vec3<f32>(CLASSIFY.size) / f32(voxels);
    let o = origin * scale;
    // Axes the ray runs parallel to are never crossed.
    let d = select(direction * scale, vec3<f32>(1e-20), abs(direction) < vec3<f32>(1e-20));
    let inverse = 1.0 / d;

    let last = vec3<f32>(cells - 1u);
    let cell = clamp(floor(o + d * t), vec3<f32>(0.0), last);
    let step = select(vec3<i32>(-1), vec3<i32>(1), d > vec3<f32>(0.0));
    let next = (cell + vec3<f32>(d > vec3<f32>(0.0)) - o) * inverse;

    return Walk(vec3<u32>(cell), step, next, abs(inverse));
}

fn walk_exit(walk: Walk) -> f32 {
    return min(walk.next.x, min(walk.next.y, walk.next.z));
}

// Leaving the grid wraps the cell past the cell count, which ends the walk.
fn walk_next(walk: Walk) -> Walk {
    let crossed = walk.next == vec3<f32>(walk_exit(walk));
    var result = walk;
    result.cell = vec3<u32>(vec3<i32>(walk.cell) + select(vec3<i32>(0), walk.step, crossed));
    result.next += select(vec3<f32>(0.0), walk.delta, crossed);
    return result;
}

// Tentative collisions at the per-block majorant. `depth` is the optical depth
// left to the next one: the caller draws it, `advance` spends it.
struct Tracker {
    walk: Walk,
    t: f32,
    exit: f32,
    depth: f32,
    // The majorant at the last collision.
    majorant: f32,
}

// The empty space in front of the material is crossed a coarse cell at a time.
fn begin_tracking(origin: vec3<f32>, direction: vec3<f32>, entry: f32, exit: f32) -> Tracker {
    var t = entry;
    var coarse = begin_walk(origin, direction, t, BLOCK * COARSE, coarse_count());

    while (t < exit && all(coarse.cell < coarse_count()) && coarse_empty(coarse.cell)) {
        t = max(t, walk_exit(coarse));
        coarse = walk_next(coarse);
    }

    let walk = begin_walk(origin, direction, t, BLOCK, block_count());
    return Tracker(walk, t, exit, 0.0, 0.0);
}

fn tracking(tracker: Tracker) -> bool {
    return tracker.t < tracker.exit && all(tracker.walk.cell < block_count());
}

// One step, kept flat so a loop over it doesn't nest: true at a tentative
// collision, false after crossing into the next block.
fn advance(tracker: ptr<function, Tracker>) -> bool {
    let boundary = min(walk_exit((*tracker).walk), (*tracker).exit);
    let majorant = block_majorant((*tracker).walk.cell, scattering_scale());

    let span = majorant * max(boundary - (*tracker).t, 0.0);

    if ((*tracker).depth >= span) {
        (*tracker).depth -= span;
        (*tracker).t = max((*tracker).t, boundary);
        (*tracker).walk = walk_next((*tracker).walk);
        return false;
    }

    (*tracker).t += (*tracker).depth / majorant;
    (*tracker).majorant = majorant;
    return true;
}

// Steps to the next tentative collision; false once the ray leaves. Faster
// than the flat loop where many blocks are empty, as along shadow rays.
fn next_collision(tracker: ptr<function, Tracker>) -> bool {
    while (tracking(*tracker)) {
        if (advance(tracker)) { return true; }
    }

    return false;
}

// Ratio tracking (Novák et al. 2014): unbiased transmittance through the
// volume over [entry, exit), times the lines' when `lines`. `dimension` picks
// the noise stream.
fn ratio_tracking(origin: vec3<f32>, direction: vec3<f32>, entry: f32, exit: f32, lines: bool, pixel: vec2<u32>, dimension: u32) -> vec3<f32> {
    var tracker = begin_tracking(origin, direction, entry, exit);
    var volume = vec3<f32>(1.0);

    var i = 0u;
    var u = flight_noise(pixel, dimension, i);
    tracker.depth = -log(1.0 - u.x);

    while (i < TRACKING_MAX && next_collision(&tracker)) {
        let extinction = to_material(classify(origin + direction * tracker.t)).extinction;
        volume *= max(tracker.majorant - extinction, vec3<f32>(0.0)) / tracker.majorant;

        let largest = max(volume.x, max(volume.y, volume.z));
        if (largest < ROULETTE) {
            if (u.y >= largest / ROULETTE) { return vec3<f32>(0.0); }
            volume *= ROULETTE / largest;
        }

        i++;
        u = flight_noise(pixel, dimension, i);
        tracker.depth = -log(1.0 - u.x);
    }

    if (!lines) { return volume; }
    return volume * line_transmittance(origin, direction, entry, exit, flight_noise(pixel, dimension + 1u, 0u).x);
}

// Line optical depth by a jittered march, a sample per voxel: unbiased, and
// with no data-dependent steps the fetches overlap. Its exponential is a close
// estimate of the transmittance. A walk that read a per-block
// line majorant every step was latency bound and over 20× slower.
fn line_transmittance(origin: vec3<f32>, direction: vec3<f32>, entry: f32, exit: f32, jitter: f32) -> f32 {
    let count = u32(ceil(max(exit - entry, 0.0) * length(direction * vec3<f32>(CLASSIFY.size))));
    let step = (exit - entry) / f32(max(count, 1u));
    var depth = 0.0;

    for (var k = 0u; k < count && depth < OPAQUE; k += 4u) {
        let t = entry + (f32(k) + jitter) * step;
        let a = line_density(origin + direction * t);
        let b = select(0.0, line_density(origin + direction * (t + step)), k + 1u < count);
        let c = select(0.0, line_density(origin + direction * (t + 2.0 * step)), k + 2u < count);
        let d = select(0.0, line_density(origin + direction * (t + 3.0 * step)), k + 3u < count);
        depth += (a + b + c + d) * step;
    }

    return exp(-depth);
}

// Optical depth past which nothing gets through.
const OPAQUE: f32 = 20.0;

fn line_density(uv: vec3<f32>) -> f32 {
    return textureSampleLevel(LINE_EXTINCTION, LINE_SAMPLER, uv, 0.0).x;
}

// Free flights must be independent: a rotated sequence ties each flight to
// the last and biases the tracking. Only the first is blue.
fn flight_noise(pixel: vec2<u32>, dimension: u32, i: u32) -> vec4<f32> {
    if (i == 0u) { return noise(pixel, dimension); }
    let seed = vec3<u32>(pixel, ENVIRONMENT.sample ^ (dimension << 24u));
    return white_noise(seed ^ (vec3<u32>(0x9e3779b9u, 0x85ebca6bu, 0xc2b2ae35u) * i));
}
