@group(0) @binding(0) var VOLUME: texture_3d<f32>;
@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(2) @binding(0) var COLOR: texture_storage_2d<bgra8unorm, write>;

@compute
@workgroup_size(64, 1)
fn main(@builtin(workgroup_id) tile: vec3<u32>, @builtin(local_invocation_index) local: u32) {
    let pixel = tile.xy * 8 + vec2<u32>(local & 7, local >> 3);

    if (any(pixel >= ENVIRONMENT.surface)) { return; }

    let result = compute(pixel);

    textureStore(COLOR, vec2<u32>(pixel.x, ENVIRONMENT.surface.y - pixel.y), result);
}

fn compute(pixel: vec2<u32>) -> vec4<f32> {
    let dim_u32 = vec3<u32>(textureDimensions(VOLUME));
    let dim = vec3<f32>(dim_u32);

    let radius = ENVIRONMENT.settings.streamline_radius / dim.x;
    let alpha = ENVIRONMENT.settings.alpha;

    let uv = vec2<f32>(pixel) / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0;

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));
    let direction = normalize(far - near);

    let value = raymarch(near + 0.5, direction, VOLUME);

    return vec4<f32>(vec3<f32>(value), 1.0);
}

fn raymarch(origin: vec3<f32>, direction: vec3<f32>, volume: texture_3d<f32>) -> f32 {
    let dim = f32(textureDimensions(volume).x);
    let dim_inv = 1.0 / dim;

    let delta = select(1.0 / direction, vec3<f32>(1E10), abs(direction) < vec3<f32>(1E-5));
    let boundary = select(vec3<u32>(0), vec3<u32>(1), direction >= vec3<f32>(0.0));

    let tMinBounds = (vec3<f32>(0.0) - origin) * delta;
    let tMaxBounds = (vec3<f32>(1.0) - origin) * delta;

    let tEnter = maximum(min(tMinBounds, tMaxBounds)) + 1E-5;
    let tExit = minimum(max(tMinBounds, tMaxBounds)) - 1E-5;

    if (tEnter >= tExit || tExit < 0.0) { return 0.0; }

    var t = max(tEnter, 0.0);
    var mip = textureNumLevels(volume) - 1;
    var position = origin + direction * t;
    var voxel = vec3<u32>(floor(position * dim));

    while (t < tExit) {
        // Get the current voxel at the current mip level
        let voxel_at_mip = voxel >> vec3<u32>(mip);

        // Traverse down level if mip is occupied
        if (textureLoad(volume, voxel_at_mip, i32(mip)).x > 0.0) {
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

    var absorbance = 0.0;

    while (t < tExit) {
        // Get the next voxel boundary
        let next = voxel + boundary;

        // Get minimum distance till next voxel boundaries
        let d = abs((vec3<f32>(next) * dim_inv - position) * delta);

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = max(d[axis], 1E-5);

        // SHADE!!
        let density = increment * dim * precision_decode(textureLoad(volume, voxel, 0).x);
        absorbance += (1.0 - absorbance) * saturate(density);

        if (absorbance > 0.99) { return 1.0; }

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    return absorbance;
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
