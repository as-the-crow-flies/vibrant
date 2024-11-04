@group(0) @binding(0) var VALUE: texture_3d<f32>;
@group(0) @binding(1) var COLOR: texture_3d<f32>;
@group(0) @binding(2) var HISTOGRAM: texture_1d<f32>;
@group(0) @binding(3) var SAMPLER: sampler;
@group(0) @binding(4) var<uniform> TRANSFORM: mat4x4<f32>;
@group(0) @binding(5) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.14159265358979323846264338327950288419716939937510582097494459230781640628620899862803482534211706798214808651328230;

struct Ray {
    @builtin(position) position: vec4<f32>,
    @location(0) origin: vec3<f32>,
    @location(1) direction: vec3<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Ray
{
    let transform = TRANSFORM * ENVIRONMENT.camera.projection_inverse;
    let near = ENVIRONMENT.camera.near;
    let far = ENVIRONMENT.camera.far;

    let uv = vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0));
    let clip = vec4<f32>(2.0 * uv - 1.0, -1.0, 1.0);

    let origin = (transform * clip * near).xyz;
    let direction = (transform * vec4(clip.xy * (far - near), far + near, far - near)).xyz;

    return Ray(vec4<f32>(clip.xy, 0.0, 1.0), origin, direction);
}

@fragment
fn fragment(ray: Ray) -> @location(0) vec4<f32> {
    let rnd = random(ray.position.xy);

    let view = normalize(TRANSFORM_INVERSE * vec4<f32>(-ray.direction, 0.0)).xzy;
    let light = normalize(TRANSFORM * vec4<f32>(ENVIRONMENT.light, 0.0)).xyz;

    let hit = intersection(ray.origin, ray.direction, vec3<f32>(0.0), vec3<f32>(1.0));

    let depth = abs(hit.y - hit.x);

    let ss = ENVIRONMENT.settings.step_size;

    let dim = vec3<f32>(textureDimensions(VALUE));

    let x = vec3<f32>(1., 0., 0.) / dim * ENVIRONMENT.settings.grad_size;
    let y = vec3<f32>(0., 1., 0.) / dim * ENVIRONMENT.settings.grad_size;
    let z = vec3<f32>(0., 0., 1.) / dim * ENVIRONMENT.settings.grad_size;

    let steps = abs(ray.direction * depth * dim);
    let step = ss / maximum(steps);

    var color = vec4<f32>(0.0);

    for (var distance = hit.x + step * rnd; distance < hit.y; distance += step) {
        let sample = ray.origin + ray.direction * distance;
        let value = volume(sample);

        if (value < ENVIRONMENT.settings.min_value) { continue; }

        let gradient = (TRANSFORM_INVERSE * -vec4<f32>(
            volume(sample + x) - volume(sample - x),
            volume(sample + y) - volume(sample - y),
            volume(sample + z) - volume(sample - z),
            0.0
        )).xyz;

        let gradient_magnitude = length(gradient);
        let normal = gradient / max(1.0, gradient_magnitude);

        let opacity = min(1.0, ss * ENVIRONMENT.settings.opacity_factor * value); // TODO: Needs Transfer Function

        let lambert = max(0.0, dot(normal, ENVIRONMENT.light));
        let specular = pow(max(0.0, dot(normalize(ENVIRONMENT.light + view), normal)), 16.0);

        // Photo-Realistic Rendering with Exposure Render
        let p_brdf = min(1.0, opacity * (1.0 - exp(-25.0 * pow(ENVIRONMENT.settings.gradient_factor, 3.0) * gradient_magnitude)));
        let p_phase = 1.0 - p_brdf;

        let total = p_brdf * (
            (1.0 - ENVIRONMENT.settings.direct_light) +
            ENVIRONMENT.settings.direct_light * (0.4 * lambert + 0.6 * specular));

        if (ENVIRONMENT.settings.ambient_occlusion_samples < 10u)
        {
            color += (1.0 - color.a) * vec4<f32>(opacity * (normal * 0.5 + 0.5), opacity);
        }
        else if (ENVIRONMENT.settings.ambient_occlusion_samples > 40u)
        {
            color += (1.0 - color.a) * vec4<f32>(vec3<f32>(shadow(sample, light)), opacity);
        }
        else
        {
            color += (1.0 - color.a) * vec4<f32>(vec3<f32>(total), opacity);
        }

        if (color.a > 0.99) { break; }
    }

    return vec4<f32>(color.xyz, 1.0);
}

fn shadow(origin: vec3<f32>, direction: vec3<f32>) -> f32 {
    let steps = abs(direction * vec3<f32>(textureDimensions(VALUE)));
    let step = 1.0 / maximum(steps);

    var occlusion = 0.0;

    for (var distance = 0.0; distance < 1.0; distance += step) {
        let sample = origin + direction * distance;
        let opacity = min(1.0, ENVIRONMENT.settings.opacity_factor * volume(sample));

        occlusion = (1.0 - occlusion) * opacity;
    }

    return occlusion;
}

fn volume(sample: vec3<f32>) -> f32 {
    return textureSampleLevel(VALUE, SAMPLER, sample, 0.0).x;
}

fn intersection(rayOrigin: vec3<f32>, rayDir: vec3<f32>, boxMin: vec3<f32>, boxMax: vec3<f32>) -> vec2<f32> {
    let tMin = (boxMin - rayOrigin) / rayDir;
    let tMax = (boxMax - rayOrigin) / rayDir;
    let t1 = min(tMin, tMax);
    let t2 = max(tMin, tMax);
    let tNear = maximum(t1);
    let tFar = minimum(t2);
    return vec2(tNear, tFar);
};

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

fn random(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}
