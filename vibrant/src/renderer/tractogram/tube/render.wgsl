@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Segment {
    @builtin(vertex_index) index: u32,
    @location(0) v0: vec3<f32>,
    @location(1) v1: vec3<f32>,
    @location(2) v2: vec3<f32>
}

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) tangent: vec3<f32>,
    @location(2) bitangent: vec3<f32>,
    @location(3) percentage: f32,
    @location(4) radius: f32,
}

struct Target {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@vertex
fn vertex(segment: Segment) -> Fragment
{
    let quad = vec2<f32>(f32((segment.index & 1) == 0), 2.0 * f32((segment.index & 2) == 0) - 1.0);
    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));

    let v0_world = TRACTOGRAM_TO_WORLD * vec4<f32>(segment.v0, 1.0);
    let v1_world = TRACTOGRAM_TO_WORLD * vec4<f32>(segment.v1, 1.0);
    let v2_world = TRACTOGRAM_TO_WORLD * vec4<f32>(segment.v2, 1.0);

    let v0 = ENVIRONMENT.camera.projection * v0_world;
    let v1 = ENVIRONMENT.camera.projection * v1_world;
    let v2 = ENVIRONMENT.camera.projection * v2_world;

    let tangent = normalize(mix(v1.xy - v0.xy, v2.xy - v1.xy, quad.x));
    let bitangent = vec2<f32>(tangent.y, -tangent.x);

    let clip = mix(v0, v1, quad.x) + vec4<f32>(quad.y * bitangent * radius, 0.0, 0.0);

    let tangent_world = normalize(TRACTOGRAM_TO_WORLD * vec4<f32>(mix(segment.v1 - segment.v0, segment.v2 - segment.v1, quad.x), 0.0));
    let bitangent_world = normalize(ENVIRONMENT.camera.projection_inverse * vec4<f32>(bitangent, 0.0, 0.0));
    let position_world = mix(v0_world, v1_world, quad.x) + quad.y * bitangent_world * radius;

    return Fragment(clip, position_world.xyz, tangent_world.xyz, bitangent_world.xyz, quad.y, radius);
}

@fragment
fn fragment(fragment: Fragment) -> Target {
    let normal = slerp(cross(fragment.tangent, fragment.bitangent), fragment.bitangent, fragment.percentage);
    let position = ENVIRONMENT.camera.projection * vec4<f32>(fragment.position + normal * fragment.radius, 1.0);
    let depth = position.z / position.w;

    let kd = ENVIRONMENT.settings.direct_light;

    let lighting = (1.0 - kd) + kd * vec3<f32>(max(0.0, dot(normal, ENVIRONMENT.light)));

    let color = vec4<f32>(lighting * abs(fragment.tangent), 1.0);
    return Target(color, depth);
}

fn slerp(start: vec3<f32>, end: vec3<f32>, percent: f32) -> vec3<f32> {
     let dot = dot(start, end);
     let theta = acos(dot) * percent;
     return start * cos(theta) + normalize(end - start * dot) * sin(theta);
}
