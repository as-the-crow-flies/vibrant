fn voxelize(index: u32, v0_: Vertex, v1_: Vertex, radius: f32) {
    let delta = v1_.xyz - v0_.xyz;
    let axes = rank(abs(delta));

    // Extend by one radius in major direction to ensure caps are voxelized
    let extension = delta / abs(delta[axes[0]]) * radius;
    let v0 = select(v1_.xyz + extension, v0_.xyz - extension, delta[axes[0]] > 0.0);
    let v1 = select(v0_.xyz - extension, v1_.xyz + extension, delta[axes[0]] > 0.0);

    // Find cylinder radii along the minor axes
    let direction = normalize(delta);
    let r1 = radius / sqrt(1.0 - direction[axes[1]] * direction[axes[1]]);
    let r2 = radius / sqrt(1.0 - direction[axes[2]] * direction[axes[2]]);

    let t_min = v0[axes[0]];
    let t_max = v1[axes[0]];

    let step = (v1 - v0) / (t_max - t_min);

    var t0 = t_min;
    var p0 = v0;

    while (t0 < t_max) {
        let t1 = min(t_max, floor(t0 + 1.0)); // Jump to the next voxel boundary or line end
        let p1 = v0 + step * (t1 - t_min); // position along line at t1

        let i = i32(t0);

        // Compute bounding square along minor axes
        let j_min = i32(min(p0[axes[1]], p1[axes[1]]) - r1);
        let j_max = i32(max(p0[axes[1]], p1[axes[1]]) + r1);

        let k_min = i32(min(p0[axes[2]], p1[axes[2]]) - r2);
        let k_max = i32(max(p0[axes[2]], p1[axes[2]]) + r2);

        // Visit all voxels in bounding square
        for (var j = j_min; j <= j_max; j++) {
            for (var k = k_min; k <= k_max; k++) {
                let voxel = shuffle(vec3<i32>(i, j, k), axes);
                visit_voxel(voxel, index, v0_, v1_);
            }
        }

        t0 = t1;
        p0 = p1;
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
