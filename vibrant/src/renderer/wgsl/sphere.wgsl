use math;

fn equirectangular(direction: vec3<f32>, rotation: f32) -> vec2<f32> {
    let d = rotation_y(rotation * 2.0 * PI) * normalize(direction);
    let u = 0.5 + atan2(d.z, d.x) / (2.0 * PI);
    let v = acos(clamp(d.y, -1.0, 1.0)) / PI;

    return vec2<f32>(u, v);
}

fn octahedron_decode(o: vec2<f32>) -> vec3<f32> {
    var v = vec3f(o.x, o.y, 1.0 - abs(o.x) - abs(o.y));
    if v.z < 0.0 { v = vec3f((1.0 - abs(v.yx)) * sign_not_zero_2(v.xy), v.z); }
    return normalize(v);
}

fn clarberg_equal_area_sphere(uv: vec2<f32>) -> vec3<f32> {
    let au = abs(uv.x);
    let av = abs(uv.y);

    // Branchless octahedral fold
    let d = 1.0 - (au + av);
    let r = 1.0 - abs(d);

    // Avoid division by zero at poles
    let inv_r = select(0.0, 1.0 / r, r > 0.0);
    let phi0 = 0.25 * PI * ((av - au) * inv_r + 1.0);

    let xy_scale = r * sqrt(max(0.0, 2.0 - r * r));

    let xy = sign_not_zero_2(uv) * vec2<f32>(cos(phi0), sin(phi0)) * xy_scale;

    let z_mag = 1.0 - r * r;
    let z = sign_not_zero(d) * z_mag;

    return vec3<f32>(xy, z);
}
