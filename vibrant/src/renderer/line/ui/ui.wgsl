@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;

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
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(1.0, -1.0) * (pixel.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));

    let origin = near;
    let direction = normalize(far - near);

    let hit = sphere(origin, direction, ENVIRONMENT.segment.position, ENVIRONMENT.segment.radius);

    if (hit.x < 0.0) { discard; }

    let position = origin + hit.x * direction;
    let normal = normalize(position - ENVIRONMENT.segment.position);

    if (abs(dot(normal, direction)) > 0.2) { discard; }

    return vec4<f32>(1.0);
}

// https://www.shadertoy.com/view/4d2XWV
fn sphere(ro: vec3<f32>, rd: vec3<f32>, ce: vec3<f32>, ra: f32) -> vec2<f32> {
    let oc = ro - ce;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - ra*ra;
    let h = b*b - c;

    if(h<0.0) { return vec2(-1.0); } // no intersection

    let h_sqrt = sqrt(h);
    return vec2(-b-h_sqrt, -b+h_sqrt);
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}
