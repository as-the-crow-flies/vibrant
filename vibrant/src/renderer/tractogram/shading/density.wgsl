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
    let step = 1.0 / maximum(abs(ray.direction * dim));
    let factor = dim.x * step;

    let jitter = random(ray.position.xy) * step;

    var occlusion = 0.0;

    for (var distance = hit.x + jitter; distance < hit.y; distance += step) {
        let position = ray.origin + distance * ray.direction;
        let density = factor * precision_decode(textureSampleLevel(DENSITY, SAMPLER, position + 0.5, ENVIRONMENT.settings.level).x);
        occlusion += (1.0 - occlusion) * saturate(density);
    }

    return vec4<f32>(vec3<f32>(occlusion), 1.0);
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

fn random(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}
