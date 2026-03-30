@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var<storage> HIGHLIGHT_DATA: array<u32>;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0
    );
}

fn unproject_h(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}

fn sphere_hit(ro: vec3<f32>, rd: vec3<f32>, ce: vec3<f32>, ra: f32) -> bool {
    let oc = ro - ce;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - ra * ra;
    let h = b * b - c;
    return h >= 0.0 && (-b + sqrt(max(0.0, h))) > 0.0;
}

fn box_hit(ro: vec3<f32>, rd: vec3<f32>, ce: vec3<f32>, hs: f32) -> bool {
    let inv = 1.0 / rd;
    let t1 = (ce - vec3<f32>(hs) - ro) * inv;
    let t2 = (ce + vec3<f32>(hs) - ro) * inv;
    let tmin = max(max(min(t1.x, t2.x), min(t1.y, t2.y)), min(t1.z, t2.z));
    let tmax = min(min(max(t1.x, t2.x), max(t1.y, t2.y)), max(t1.z, t2.z));
    return tmax >= 0.0 && tmin <= tmax;
}

fn rectangle_hit(ro: vec3<f32>, rd: vec3<f32>, ce: vec3<f32>, hs: vec3<f32>) -> bool {
    let inv = 1.0 / rd;
    let t1 = (ce - hs - ro) * inv;
    let t2 = (ce + hs - ro) * inv;
    let tmin = max(max(min(t1.x, t2.x), min(t1.y, t2.y)), min(t1.z, t2.z));
    let tmax = min(min(max(t1.x, t2.x), max(t1.y, t2.y)), max(t1.z, t2.z));
    return tmax >= 0.0 && tmin <= tmax;
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(1.0, -1.0) * (pixel.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);
    let ro = unproject_h(vec3<f32>(uv, 0.0));
    let rd = normalize(unproject_h(vec3<f32>(uv, 1.0)) - ro);

    let count = HIGHLIGHT_DATA[0];

    for (var i = 0u; i < count; i++) {
        let shape  = HIGHLIGHT_DATA[1u + i * 9u];
        let scale  = bitcast<f32>(HIGHLIGHT_DATA[2u + i * 9u]);
        let x      = bitcast<f32>(HIGHLIGHT_DATA[3u + i * 9u]);
        let y      = bitcast<f32>(HIGHLIGHT_DATA[4u + i * 9u]);
        let z      = bitcast<f32>(HIGHLIGHT_DATA[5u + i * 9u]);
        let negate = HIGHLIGHT_DATA[6u + i * 9u];
        let size_x = bitcast<f32>(HIGHLIGHT_DATA[7u + i * 9u]);
        let size_y = bitcast<f32>(HIGHLIGHT_DATA[8u + i * 9u]);
        let size_z = bitcast<f32>(HIGHLIGHT_DATA[9u + i * 9u]);

        let ce = vec3<f32>(x, y, z);
        let hs = 0.125 * scale;

        var hit = false;
        if shape == 0u {
            hit = box_hit(ro, rd, ce, hs);
        } else if shape == 2u {
            hit = rectangle_hit(ro, rd, ce, vec3<f32>(size_x, size_y, size_z));
        } else {
            hit = sphere_hit(ro, rd, ce, hs);
        }

        if hit {
            let color = select(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(1.0, 0.2, 0.2), negate == 1u);
            let alpha = select(0.15, 0.1, negate == 1u);
            return vec4<f32>(color, alpha);
        }
    }

    return vec4<f32>(0.0);
}
