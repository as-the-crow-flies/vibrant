@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;

@group(1) @binding(0) var DEPTH: texture_depth_2d;
@group(1) @binding(1) var INDEX: texture_2d<u32>;

@group(2) @binding(1) var SAMPLER: sampler;
@group(2) @binding(4) var OCCLUSION_AMBIENT: texture_3d<f32>;
@group(2) @binding(6) var OCCLUSION_DIRECTIONAL: texture_3d<f32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

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
fn fragment(@builtin(position) clip: vec4<f32>) -> @location(0) vec4<f32> {
    let radius = ENVIRONMENT.settings.radius / f32(ENVIRONMENT.volume);

    let pixel = vec2<u32>(clip.xy);

    let depth = textureLoad(DEPTH, pixel, 0);

    if (depth == 1.0) { discard; }

    let index = textureLoad(INDEX, pixel, 0).x;

    let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
    let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

    let uv = vec2<f32>(1.0, -1.0) * (clip.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);
    let position = unproject(vec3<f32>(uv, depth));

    let color = shade(
        v0, v1, radius, position,
        ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER
    );

    return color;
}


fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}
