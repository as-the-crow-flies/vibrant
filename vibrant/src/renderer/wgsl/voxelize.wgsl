fn voxelize(index: u32, v0_: vec3<f32>, v1_: vec3<f32>, radius: f32) {
    let direction = normalize(v1_ - v0_);
    let axes = rank(abs(direction));
    let r = radius * direction;

    let v0 = select(v1_ + r, v0_ - r, direction[axes[0]] > 0.0);
    let v1 = select(v0_ - r, v1_ + r, direction[axes[0]] > 0.0);

    let t_min = v0[axes[0]];
    let t_max = v1[axes[0]];

    let step = (v1 - v0) / (t_max - t_min);

    var t0 = t_min;
    var s0 = v0;

    while (t0 < t_max) {
        let t1 = min(t_max, floor(t0 + 1.0));
        let s1 = v0 + step * (t1 - t_min);

        let i = i32(t0);

        let j_min = i32(min(s0[axes[1]], s1[axes[1]]) - radius);
        let j_max = i32(max(s0[axes[1]], s1[axes[1]]) + radius);

        let k_min = i32(min(s0[axes[2]], s1[axes[2]]) - radius);
        let k_max = i32(max(s0[axes[2]], s1[axes[2]]) + radius);

        for (var j = j_min; j <= j_max; j++) {
            for (var k = k_min; k <= k_max; k++) {
                let voxel = shuffle(vec3<i32>(i, j, k), axes);

                // TODO: check if it actually hits corner voxels! (maybe it does not?)
                visit_voxel(voxel, index, v0, v1);
            }
        }

        t0 = t1;
        s0 = s1;
    }
}

fn shuffle(v: vec3<i32>, axes: vec3<i32>) -> vec3<i32> {
    var result = vec3<i32>();
    result[axes[0]] = v[0];
    result[axes[1]] = v[1];
    result[axes[2]] = v[2];
    return result;
}

fn rank(v: vec3<f32>) -> vec3<i32> {
    var val = v;
    var idx = vec3<i32>(0, 1, 2);

    if (val.x < val.y) {
        val = val.yxz;
        idx = idx.yxz;
    }
    if (val.y < val.z) {
        val = val.xzy;
        idx = idx.xzy;
    }
    if (val.x < val.y) {
        val = val.yxz;
        idx = idx.yxz;
    }

    return idx;
}
