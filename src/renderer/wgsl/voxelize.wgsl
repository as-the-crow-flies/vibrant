use vertex;

// Visits every voxel touched by the capsule of `radius` around v0-v1. Works in
// a frame where x is the segment's major axis, sweeping one x-slab at a time.
fn voxelize(index: u32, v0: Vertex, v1: Vertex, radius: f32) {
    let axes = rank(abs(v1.xyz - v0.xyz));
    let a = permute(v0.xyz, axes);
    let b = permute(v1.xyz, axes);
    let d = b - a;
    let c = capsule(a, d, radius);

    let i_min = i32(floor(min(a.x, b.x) - radius));
    let i_max = i32(floor(max(a.x, b.x) + radius));

    for (var i = i_min; i <= i_max; i++) {
        // The part of the segment within `radius` of this slab.
        var t = vec2<f32>(0.0, 1.0);
        if (d.x != 0.0) {
            let t0 = (f32(i) - radius - a.x) / d.x;
            let t1 = (f32(i) + 1.0 + radius - a.x) / d.x;
            t = saturate(vec2<f32>(min(t0, t1), max(t0, t1)));
        }

        let p = a + d * t.x;
        let q = a + d * t.y;

        let lo = vec2<i32>(floor(min(p.yz, q.yz) - radius));
        let hi = vec2<i32>(floor(max(p.yz, q.yz) + radius));

        for (var j = lo.x; j <= hi.x; j++) {
            for (var k = lo.y; k <= hi.y; k++) {
                let voxel = vec3<i32>(i, j, k);

                if (touches(c, vec3<f32>(voxel))) {
                    visit_voxel(unpermute(voxel, axes), index, v0, v1);
                }
            }
        }
    }
}

// Conservative capsule-voxel overlap test: rejects a voxel only on a
// separating axis, from the tube's silhouette edges (d × eᵢ) or the
// closest-point direction.
struct Capsule {
    a: vec3<f32>,
    d: vec3<f32>,
    radius: f32,
    edge_limit: vec3<f32>,
}

fn capsule(a: vec3<f32>, d: vec3<f32>, radius: f32) -> Capsule {
    let edge_limit = vec3<f32>(
        extent(vec3<f32>(0.0, d.z, -d.y), radius),
        extent(vec3<f32>(-d.z, 0.0, d.x), radius),
        extent(vec3<f32>(d.y, -d.x, 0.0), radius),
    );
    return Capsule(a, d, radius, edge_limit);
}

fn touches(c: Capsule, voxel: vec3<f32>) -> bool {
    let offset = voxel + 0.5 - (c.a + 0.5 * c.d);

    // offset · (d × eᵢ) for all three edges at once.
    if (any(abs(cross(offset, c.d)) > c.edge_limit)) { return false; }

    let p = c.a + c.d * saturate(dot(voxel + 0.5 - c.a, c.d) / max(dot(c.d, c.d), 1e-12));
    let n = p - clamp(p, voxel, voxel + 1.0);

    return abs(dot(offset, n)) <= 0.5 * abs(dot(c.d, n)) + extent(n, c.radius);
}

// Half-extent along `n` of a voxel plus a ball of `radius`.
fn extent(n: vec3<f32>, radius: f32) -> f32 {
    return radius * length(n) + 0.5 * dot(abs(n), vec3<f32>(1.0));
}

// Axis indices sorted by descending value.
fn rank(v: vec3<f32>) -> vec3<u32> {
    if (v.x >= v.y && v.x >= v.z) { return select(vec3<u32>(0u, 2u, 1u), vec3<u32>(0u, 1u, 2u), v.y >= v.z); }
    if (v.y >= v.z) { return select(vec3<u32>(1u, 2u, 0u), vec3<u32>(1u, 0u, 2u), v.x >= v.z); }
    return select(vec3<u32>(2u, 1u, 0u), vec3<u32>(2u, 0u, 1u), v.x >= v.y);
}

fn permute(v: vec3<f32>, axes: vec3<u32>) -> vec3<f32> {
    return vec3<f32>(v[axes.x], v[axes.y], v[axes.z]);
}

fn unpermute(v: vec3<i32>, axes: vec3<u32>) -> vec3<i32> {
    var result: vec3<i32>;
    result[axes.x] = v.x;
    result[axes.y] = v.y;
    result[axes.z] = v.z;
    return result;
}
