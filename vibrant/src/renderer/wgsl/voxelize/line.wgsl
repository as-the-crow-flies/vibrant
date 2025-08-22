fn voxelize(index: u32, v0: Vertex, v1: Vertex, radius: f32) {
    let delta = v1.xyz - v0.xyz;
    let distance = length(delta);
    let direction = delta / distance;
    let voxel_boundaries = 1.0 / abs(direction);
    let step = vec3<i32>(sign(direction));
    var next = vec4<f32>(
        one_if_zero(abs(fract(vec3<f32>(-step) * fract(v0.xyz)))) * voxel_boundaries,
        distance
    );

    var voxel = vec3<i32>(v0.xyz);

    while (next.w > 0.0) {
        let increment = minimum(next);

        visit_voxel_line(voxel, index, v0, v1, increment);

        let mask = next == vec4<f32>(increment);
        voxel += step * vec3<i32>(mask.xyz);
        next = select(
            next - increment,
            vec4<f32>(voxel_boundaries, 0.0),
            mask
        );
    }
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v == vec3<f32>(0.0));
}

fn minimum(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}
