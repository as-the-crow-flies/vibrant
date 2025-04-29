@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(1) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(2) @binding(0) var DENSITY: texture_3d<f32>;
@group(2) @binding(1) var DENSITY_SAMPLER: sampler;

@group(3) @binding(0) var OCCLUSION: texture_3d<f32>;
@group(3) @binding(1) var OCCLUSION_SAMPLER: sampler;

@group(4) @binding(0) var<uniform> ENVIRONMENT: Environment;

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

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) tangent: vec3<f32>,
    @location(2) @interpolate(flat) v0: vec3<f32>,
    @location(3) @interpolate(flat) v1: vec3<f32>,
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

    let radius = ENVIRONMENT.settings.streamline_radius / f32(ENVIRONMENT.volume);
    let vertex = CUBE[vertex_index] * vec3<f32>(radius, distance + 2.0 * radius, radius);

    let position = v0 + vertex.x * dx + vertex.y * dy - radius * dy + vertex.z * dz;
    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);

    let tangent = abs(normalize(select(v1 - v0, v2 - v1, CUBE[vertex_index].y == 1.0 && v2.x < 1E9)));

    return Fragment(clip, position, tangent, v0, v1);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let radius = ENVIRONMENT.settings.streamline_radius / f32(ENVIRONMENT.volume);

    let origin = ENVIRONMENT.camera.transform[3].xyz;
    let direction = normalize(fragment.position - origin);

    let front = capsule_intersection(origin, direction, fragment.v0, fragment.v1, radius);

    if (front < 0.0) { discard; }

    let sphere = sphere_intersection(origin, direction, fragment.v1, radius);

    if (sphere.x >= 0) { discard; }

    let position = origin + front * direction;
    let normal = capsule_normal(position, fragment.v0, fragment.v1, radius);

    var clip = ENVIRONMENT.camera.projection * vec4<f32>(position + direction * radius, 1.0);
    clip /= clip.w;

    let surface_dim = vec2<f32>(ENVIRONMENT.surface);
    let occlusion_dim = vec2<f32>(textureDimensions(OCCLUSION, 0).xy * ENVIRONMENT.tile);

    let sample = vec3<f32>((0.5 + 0.5 * clip.xy) * surface_dim / occlusion_dim, linearize_depth(clip.z));
    let transmittance = 1.0 - textureSampleLevel(OCCLUSION, OCCLUSION_SAMPLER, sample, 0.0).x;

    if (transmittance < 1E-6) { discard; }

    let tangent_object_space = normalize(TRACTOGRAM_TO_WORLD * vec4<f32>(fragment.tangent, 0.0)).xyz;

    let color = abs(tangent_object_space);

    let alpha = ENVIRONMENT.settings.alpha * transmittance;

    return vec4<f32>(color * alpha, alpha);
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

fn sphere_intersection(ro: vec3<f32>, rd: vec3<f32>, ce: vec3<f32>, ra: f32) -> vec2<f32>
{
    let oc = ro - ce;
    let b = dot( oc, rd );
    let c = dot( oc, oc ) - ra*ra;
    var h = b*b - c;

    if( h<0.0 ) { return vec2(-1.0); } // no intersection

    h = sqrt( h );
    return vec2<f32>(-b-h, -b+h);
}

fn linearize_depth(ndc_depth: f32) -> f32 {
    let near = ENVIRONMENT.camera.near;
    let far = ENVIRONMENT.camera.far;

    // Convert NDC depth [0, 1] to clip-space Z [-1, 1]
    let z = ndc_depth * 2.0 - 1.0;

    // Reverse the projection to get view-space Z
    let view_z = (2.0 * near * far) / (far + near - z * (far - near));

    // Convert view-space Z to linear depth in [0, 1]
    return (view_z - near) / (far - near);
}
