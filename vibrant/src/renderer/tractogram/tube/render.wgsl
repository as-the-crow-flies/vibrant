@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Segment {
    @builtin(vertex_index) index: u32,
    @location(0) start: vec3<f32>,
    @location(1) end: vec3<f32>
}

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) quad: vec2<f32>,
    @location(1) tangent: vec3<f32>,
    @location(2) bitangent: vec3<f32>,
    @location(3) percentage: f32,
    @location(4) radius: f32
}

struct Target {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@vertex
fn vertex(segment: Segment) -> Fragment
{
    let quad = vec2<f32>(f32((segment.index & 1) == 0), 2.0 * f32((segment.index & 2) == 0) - 1.0);

    let start = transform(segment.start);
    let end = transform(segment.end);

    if (start.z < 0.0 || end.z < 0.0 || start.z > 1.0 || end.z > 1.0) { return Fragment(); }

    let tangent = normalize(end.xy - start.xy);
    let bitangent = vec2<f32>(tangent.y, -tangent.x);

    let height = mix(start, end, quad.x);
    let radius = length(TRACTOGRAM_TO_WORLD[0]) * ENVIRONMENT.settings.streamline_radius / height.w;
    let width = vec4<f32>(quad.y * vec3<f32>(bitangent, 0.0) * radius, 1.0);
    let clip = vec4<f32>(height.xyz + width.xyz, 1.0);

    let tangent_world = normalize(TRACTOGRAM_TO_WORLD * vec4<f32>(segment.end - segment.start, 0.0));
    let bitangent_world = normalize(ENVIRONMENT.camera.projection_inverse * vec4<f32>(bitangent, 0.0, 0.0));

    return Fragment(clip, quad, tangent_world.xyz, bitangent_world.xyz, quad.y, radius);
}

@fragment
fn fragment(fragment: Fragment) -> Target {
    let normal_world = slerp(cross(fragment.tangent, fragment.bitangent), fragment.bitangent, fragment.percentage);
    let normal_clip = ENVIRONMENT.camera.projection * vec4<f32>(normal_world, 0.0);

    let kd = ENVIRONMENT.settings.direct_light;

    let lighting = (1.0 - kd) + kd * vec3<f32>(max(0.0, dot(normal_world, ENVIRONMENT.light)));

    let color = vec4<f32>(lighting * abs(fragment.tangent), 1.0);
    // let depth = fragment.clip.z - normal_clip.z / normal_clip.w * fragment.radius;
    let depth = fragment.clip.z;

    return Target(color, depth);
}

fn transform(vertex: vec3<f32>) -> vec4<f32> {
    let v = ENVIRONMENT.camera.projection * TRACTOGRAM_TO_WORLD * vec4<f32>(vertex, 1.0);
    return vec4<f32>(v.xyz / v.w, v.w);
}

fn slerp(start: vec3<f32>, end: vec3<f32>, percent: f32) -> vec3<f32> {
     let dot = dot(start, end);
     let theta = acos(dot) * percent;
     return start * cos(theta) + normalize(end - start * dot) * sin(theta);
}
