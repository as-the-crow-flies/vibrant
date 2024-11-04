@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Segment {
    @builtin(vertex_index) index: u32,
    @location(0) v0: vec3<f32>,
    @location(1) v1: vec3<f32>,
    @location(2) v2: vec3<f32>,
}

struct SegmentFragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) quad: vec2<f32>,
    @location(5) radius: f32,
}

struct Cap {
    @builtin(vertex_index) index: u32,
    @location(0) v0: vec3<f32>,
    @location(1) v1: vec3<f32>,
}

struct CapFragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) quad: vec2<f32>,
}

struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@vertex
fn segment_vertex(segment: Segment) -> SegmentFragment {
    let quad = vec2<f32>(f32((segment.index & 1) == 0), 2.0 * f32((segment.index & 2) == 0) - 1.0);
    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));

    let v0 = transform(TRACTOGRAM_TO_WORLD, segment.v0);
    let v1 = transform(TRACTOGRAM_TO_WORLD, segment.v1);
    let v2 = transform(TRACTOGRAM_TO_WORLD, segment.v2);

    let t0 = normalize(v1 - v0);
    let t1 = normalize(v2 - v1);

    let camera = ENVIRONMENT.camera.transform[3].xyz;

    let line = mix(v0, v1, quad.x).xyz;
    let view = normalize(camera - line);

    let tangent = normalize(mix(t0, t1, quad.x));
    let bitangent = normalize(cross(view, tangent));
    let normal = normalize(cross(tangent, bitangent));

    let position = line + bitangent * quad.y * radius;
    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);

    let tangent_screen = (ENVIRONMENT.camera.projection * vec4<f32>(tangent, 0.0)).xyz;

    return SegmentFragment(
        clip,
        position,
        normal,
        tangent,
        bitangent,
        quad,
        radius);
}

@fragment
fn segment_fragment(fragment: SegmentFragment, @builtin(front_facing) front: bool) -> GBuffer {
    let normal = slerp(fragment.normal, fragment.bitangent, fragment.quad.y);
    let position = ENVIRONMENT.camera.projection * vec4<f32>(fragment.position + normal * fragment.radius, 1.0);
    let depth = position.z / position.w;

    return GBuffer(
        vec4<f32>(fragment.position, 1.0),
        vec4<f32>(normal, 1.0),
        vec4<f32>(fragment.tangent, 1.0),
        depth
    );
}

@vertex
fn cap_vertex(cap: Cap) -> CapFragment {
    let quad = 2.0 * vec2<f32>(f32((cap.index & 1) == 0), f32((cap.index & 2) == 0)) - 1.0;
    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));

    let v0_world = TRACTOGRAM_TO_WORLD * vec4<f32>(cap.v0, 1.0);
    let v1_world = TRACTOGRAM_TO_WORLD * vec4<f32>(cap.v1, 1.0);

    let normal = normalize(v0_world - v1_world).xyz;
    let tangent = normalize(cross(normal, vec3<f32>(1.0, 0.0, 0.0)));
    let bitangent = normalize(cross(normal, tangent));

    let position = v0_world.xyz + radius * (tangent * quad.x + bitangent * quad.y);

    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);

    return CapFragment(clip, normal, quad);
}

@fragment
fn cap_fragment(fragment: CapFragment) -> @location(0) vec4<f32> {
    if (length(fragment.quad) >= 1.0) { discard; }

    let kd = ENVIRONMENT.settings.direct_light;
    let lighting = (1.0 - kd) + kd * vec3<f32>(max(0.0, dot(fragment.normal, ENVIRONMENT.light)));
    return vec4<f32>(lighting * abs(fragment.normal), 1.0);
}

fn transform(mat: mat4x4<f32>, vec: vec3<f32>) -> vec3<f32> {
    return (mat * vec4<f32>(vec, 1.0)).xyz;
}

fn rotate(mat: mat4x4<f32>, vec: vec3<f32>) -> vec3<f32> {
    return (mat * vec4<f32>(vec, 0.0)).xyz;
}

fn align(v: vec3<f32>) -> vec3<f32> {
    return v * sign(dot(v, vec3<f32>(1.0, 1.0, 1.0)));
}

fn slerp(start: vec3<f32>, end: vec3<f32>, percent: f32) -> vec3<f32> {
     let dot = dot(start, end);
     let theta = acos(dot) * percent;
     return start * cos(theta) + normalize(end - start * dot) * sin(theta);
}
