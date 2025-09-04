@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(0) @binding(4) var OCCLUSION_AMBIENT: texture_3d<f32>;
@group(0) @binding(6) var OCCLUSION_DIRECTIONAL: texture_3d<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

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
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let dim_u32 = vec3<u32>(textureDimensions(DENSITY));
    let dim = vec3<f32>(dim_u32);

    let radius = ENVIRONMENT.settings.radius / dim.x;
    let alpha = ENVIRONMENT.settings.alpha;

    let uv = vec2<f32>(1.0, -1.0) * (pixel.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));
    let direction = normalize(far - near);

    let random = hash(uv);

    return raymarch(near + 0.5, direction, random);
}

fn raymarch(origin: vec3<f32>, direction: vec3<f32>, random: f32) -> vec4<f32> {
    let dim = f32(textureDimensions(DENSITY).x);
    let dim_inv = 1.0 / dim;

    let delta = select(1.0 / direction, vec3<f32>(1E10), abs(direction) < vec3<f32>(1E-5));
    let boundary = select(vec3<u32>(0), vec3<u32>(1), direction >= vec3<f32>(0.0));

    let tMinBounds = (vec3<f32>(0.0) - origin) * delta;
    let tMaxBounds = (vec3<f32>(1.0) - origin) * delta;

    let tEnter = maximum(min(tMinBounds, tMaxBounds)) + 1E-5;
    let tExit = minimum(max(tMinBounds, tMaxBounds)) - 1E-5;

    if (tEnter >= tExit || tExit < 0.0) { return vec4<f32>(0.0); }

    var t = max(tEnter, 0.0);
    var mip = textureNumLevels(DENSITY) - 1;
    var position = origin + direction * t;
    var voxel = vec3<u32>(floor(position * dim));

    while (t < tExit) {
        // Get the current voxel at the current mip level
        let voxel_at_mip = voxel >> vec3<u32>(mip);

        // Traverse down level if mip is occupied
        if (textureLoad(DENSITY, voxel_at_mip, i32(mip)).a > 0.0) {
            if (mip == 0) { break; }
            else { mip--; }

            continue;
        }

        // Get the next voxel boundary, given current mip level
        let next = (voxel_at_mip + boundary) << vec3<u32>(mip);

        // Get minimum distance till next voxel boundaries
        let d = abs((vec3<f32>(next) * dim_inv - position) * delta);

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = max(d[axis], 1E-5);

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel = vec3<u32>(floor(position * dim));
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    var color = vec4<f32>(0.0);

    for (t -= dim_inv * random; t < tExit; t += dim_inv) {
        let position = origin + direction * t;

        let ambient = saturate(1.0 - textureSampleLevel(OCCLUSION_AMBIENT, SAMPLER, position, 0.0).x);
        let directional = saturate(1.0 - textureSampleLevel(OCCLUSION_DIRECTIONAL, SAMPLER, position, 0.0).x);

        let factor = mix(1.0, mix(ambient, directional,
            ENVIRONMENT.settings.direct_light),
            ENVIRONMENT.settings.lighting);

        let alpha = saturate(ENVIRONMENT.settings.alpha * textureSampleLevel(DENSITY, SAMPLER, position, 0.0).x);

        let rgba = vec4<f32>(vec3<f32>(factor * alpha), alpha);

        color += (1.0 - color.a) * rgba;

        if (color.a > 0.99) { return color; }
    }

    return color;
}

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

fn hash(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}
