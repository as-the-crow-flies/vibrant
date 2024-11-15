@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Ray {
    @builtin(position) position: vec4<f32>,
    @location(0) origin: vec3<f32>,
    @location(1) direction: vec3<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Ray
{
    let quad = vec2<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2))
    );

    let near = unproject(vec3<f32>(quad, 0.0));
    let far = unproject(vec3<f32>(quad, 1.0));
    let direction = normalize(far - near);

    return Ray(vec4<f32>(quad, 0.0, 1.0), near, direction);
}

@fragment
fn fragment(ray: Ray) -> @location(0) vec4<f32> {
    let hit = aabb(ray, vec3<f32>(-0.5), vec3<f32>(0.5));
    let path = hit.y - hit.x;

    if (path <= 0) { discard; }

    let dim = vec3<f32>(textureDimensions(DENSITY));
    let step = 1.0 / maximum(abs(ray.direction * path * dim));

    var occlusion = 0.0;
    var distance = hit.x;

    for (; distance < hit.y && occlusion < 0.99; distance += step) {
        let position = ray.origin + distance * ray.direction;
        let density = textureSampleLevel(DENSITY, SAMPLER, position + 0.5, 0.0).x;
        occlusion += (1.0 - occlusion) * density;
    }

    return vec4<f32>(vec3<f32>(distance), 1.0);
}

fn aabb(ray: Ray, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> vec2<f32> {
    let tMin = (aabb_min - ray.origin) / ray.direction;
    let tMax = (aabb_max - ray.origin) / ray.direction;
    let t1 = min(tMin, tMax);
    let t2 = max(tMin, tMax);
    let tNear = max(maximum(t1), 0.0);
    let tFar = minimum(t2);
    return vec2(tNear, tFar);
};

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}
