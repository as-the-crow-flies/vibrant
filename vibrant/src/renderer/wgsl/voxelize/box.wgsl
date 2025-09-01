const SQRT_3_DIV_2: f32 = 0.8660254038;

fn voxelize(index: u32, v0: Vertex, v1: Vertex, radius: f32) {
    let aabb_min = vec3<i32>(min(v0.xyz, v1.xyz) - radius);
    let aabb_max = vec3<i32>(max(v0.xyz, v1.xyz) + radius);

    for (var x = aabb_min.x; x <= aabb_max.x; x++) {
        for (var y = aabb_min.y; y <= aabb_max.y; y++) {
            for (var z = aabb_min.z; z <= aabb_max.z; z++) {
                let voxel = vec3<i32>(x, y, z);
                let sample = vec3<f32>(voxel) + 0.5;

                let sdf = capsule_box(sample, v0.xyz, v1.xyz, radius);

                if (sdf <= SQRT_3_DIV_2) {
                    visit_voxel(voxel, index, v0, v1);
                }
            }
        }
    }
}

fn capsule_box(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - r;
}
