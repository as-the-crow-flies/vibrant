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
    let radius = ENVIRONMENT.settings.radius;
    let scale = 1.0 / f32(ENVIRONMENT.volume);

    let pixel = vec2<u32>(clip.xy);

    let depth = textureLoad(DEPTH, pixel, 0);

    if (depth == 1.0) { discard; }

    let index = textureLoad(INDEX, pixel, 0).x;

    let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
    let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

    let uv = vec2<f32>(1.0, -1.0) * (clip.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);
    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));

    let origin = (ENVIRONMENT.camera.transform[3].xyz + 0.5) * f32(ENVIRONMENT.volume);
    let direction = normalize(far - near);

    // let position = origin + direction * get_view_depth(depth) + 0.5;
    // let position_voxel = position * f32(ENVIRONMENT.volume);

    let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, radius);
    let position_voxel = origin + hit * direction;
    let position = position_voxel * scale;

    // let color = vec4<f32>(vec3<f32>(get_view_depth(depth)), 1.0);
    // let color = vec4<f32>(abs(normalize(v1.xyz - v0.xyz)), 1.0);

    let color = shade(
        v0, v1, radius, position_voxel, direction, position,
        ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);

    return color;
}

fn get_view_depth(ndc_depth: f32) -> f32 {
    let near = ENVIRONMENT.camera.near;
    let far = ENVIRONMENT.camera.far;

    // Convert NDC depth [0, 1] to clip-space Z [-1, 1]
    let z = ndc_depth * 2.0 - 1.0;

    // Reverse the projection to get view-space Z
    return (2.0 * near * far) / (far + near - z * (far - near));
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}
