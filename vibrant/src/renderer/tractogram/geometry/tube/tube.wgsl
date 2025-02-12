const CUBE: array<vec3<f32>, 14> = array(
    vec3<f32>(-1.0, 1.0, 1.0),  // Front-top-left
    vec3<f32>( 1.0, 1.0, 1.0),  // Front-top-right
    vec3<f32>(-1.0, 0.0, 1.0),  // Front-bottom-left
    vec3<f32>( 1.0, 0.0, 1.0),  // Front-bottom-right
    vec3<f32>( 1.0, 0.0,-1.0),  // Back-bottom-right
    vec3<f32>( 1.0, 1.0, 1.0),  // Front-top-right
    vec3<f32>( 1.0, 1.0,-1.0),  // Back-top-right
    vec3<f32>(-1.0, 1.0, 1.0),  // Front-top-left
    vec3<f32>(-1.0, 1.0,-1.0),  // Back-top-left
    vec3<f32>(-1.0, 0.0, 1.0),  // Front-bottom-left
    vec3<f32>(-1.0, 0.0,-1.0),  // Back-bottom-left
    vec3<f32>( 1.0, 0.0,-1.0),  // Back-bottom-right
    vec3<f32>(-1.0, 1.0,-1.0),  // Back-top-left
    vec3<f32>( 1.0, 1.0,-1.0)   // Back-top-right
);

const U32_MAX: u32 = 4294967295;

@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(1) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) tangent: vec3<f32>,
    @location(2) @interpolate(flat) v0: vec3<f32>,
    @location(3) @interpolate(flat) v1: vec3<f32>,
    @location(4) @interpolate(flat) radius: f32,
}

struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> Fragment {
    let index = TRACTOGRAM_INDICES[instance_index];

    let v0 = transform(TRACTOGRAM_TO_WORLD, TRACTOGRAM_VERTICES[index + 0]);
    let v1 = transform(TRACTOGRAM_TO_WORLD, TRACTOGRAM_VERTICES[index + 1]);
    let v2 = transform(TRACTOGRAM_TO_WORLD, TRACTOGRAM_VERTICES[index + 2]);

    let delta = v1 - v0;
    let distance = length(delta);

    let dy = delta / distance;
    let dx = normalize(cross(dy, vec3<f32>(1.0, 0.0, 0.0)));
    let dz = normalize(cross(dy, dx));

    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));
    let vertex = CUBE[vertex_index] * vec3<f32>(radius, distance + 2.0 * radius, radius);

    let position = v0 + vertex.x * dx + vertex.y * dy - radius * dy + vertex.z * dz;
    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);

    let tangent = abs(normalize(select(v1 - v0, v2 - v1, CUBE[vertex_index].y == 1.0 && v2.x < 1E9)));

    return Fragment(clip, position, tangent, v0, v1, radius);
}

@fragment
fn fragment(fragment: Fragment) -> GBuffer {
    let origin = ENVIRONMENT.camera.transform[3].xyz;
    let direction = normalize(fragment.position - origin);

    let hit = capsule_intersection(origin, direction, fragment.v0, fragment.v1, fragment.radius);

    if (hit < 0.0) { discard; }

    let position = origin + hit * direction;
    let normal = capsule_normal(position, fragment.v0, fragment.v1, fragment.radius);

    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);
    let depth = clip.z / clip.w;

    return GBuffer(
        vec4<f32>(position, 1.0),
        vec4<f32>(0.5 + 0.5 * normal, 1.0),
        vec4<f32>(fragment.tangent, 1.0),
        depth);
}

fn transform(mat: mat4x4<f32>, vec: vec4<f32>) -> vec3<f32> {
    return (mat * vec).xyz;
}

// https://iquilezles.org/articles/intersectors
fn capsule_intersection(ro: vec3<f32>, rd: vec3<f32>, pa: vec3<f32>, pb: vec3<f32>, r: f32) -> f32
{
    let ba = pb - pa;
    let oa = ro - pa;

    let baba = dot(ba,ba);
    let bard = dot(ba,rd);
    let baoa = dot(ba,oa);
    let rdoa = dot(rd,oa);
    let oaoa = dot(oa,oa);

    var a = baba      - bard*bard;
    var b = baba*rdoa - baoa*bard;
    var c = baba*oaoa - baoa*baoa - r*r*baba;
    var h = b*b - a*c;

    if (h>=0.0)
    {
        let t = (-b-sqrt(h))/a;
        let y = baoa + t*bard;

        // body
        if( y>0.0 && y<baba ) { return t; }

        // caps
        let oc = select(ro - pb, oa, y <= 0.0);

        b = dot(rd, oc);
        c = dot(oc, oc) - r*r;
        h = b*b - c;
        if (h>0.0) { return -b - sqrt(h); }
    }

    return -1.0;
}

fn capsule_normal(pos: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> vec3<f32>
{
    let ba = b - a;
    let pa = pos - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return (pa - h*ba) / r;
}
