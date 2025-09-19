@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(0) @binding(2) var COUNT: texture_3d<u32>;
@group(0) @binding(4) var OCCLUSION_AMBIENT: texture_3d<f32>;
@group(0) @binding(6) var OCCLUSION_DIRECTIONAL: texture_3d<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

var<private> DIM: f32;
var<private> DIM_INV: f32;
var<private> DIR_INV: f32;
var<private> RADIUS: f32;
var<private> ALPHA: f32;

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
    DIM = f32(ENVIRONMENT.volume);
    DIM_INV = 1.0 / DIM;
    RADIUS = ENVIRONMENT.settings.radius * DIM_INV;
    ALPHA = ENVIRONMENT.settings.alpha;

    let uv = vec2<f32>(1.0, -1.0) * (pixel.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));

    let origin = near + 0.5; // 0-1 Space
    let direction = normalize(far - near);

    return raymarch(origin, direction);
}

fn raymarch(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    let delta = select(1.0 / direction, vec3<f32>(1E10), abs(direction) < vec3<f32>(1E-5));
    let boundary = select(vec3<u32>(0), vec3<u32>(1), direction >= vec3<f32>(0.0));

    let tMinBounds = (vec3<f32>(0.0) - origin) * delta;
    let tMaxBounds = (vec3<f32>(1.0) - origin) * delta;

    let t_min = maximum(min(tMinBounds, tMaxBounds)) + 1E-5;
    let t_max = minimum(max(tMinBounds, tMaxBounds)) - 1E-5;

    if (t_min >= t_max || t_max < 0.0) { return vec4<f32>(0.0); }

    var t = max(t_min, 0.0);
    var mip = textureNumLevels(DENSITY) - 1;
    var position = origin + direction * t;
    var voxel = vec3<u32>(floor(position * DIM));

    while (t < t_max) {
        // Get the current voxel at the current mip level
        let voxel_at_mip = voxel >> vec3<u32>(mip);

        // Traverse down level if mip is occupied
        if (textureLoad(DENSITY, voxel_at_mip, i32(mip)).x > 0.0) {
            if (mip == 0) { break; }
            else { mip--; }

            continue;
        }

        // Get the next voxel boundary, given current mip level
        let next = (voxel_at_mip + boundary) << vec3<u32>(mip);

        // Get minimum distance till next voxel boundaries
        let d = abs((vec3<f32>(next) * DIM_INV - position) * delta);

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = max(d[axis], 1E-5);

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel = vec3<u32>(floor(position * DIM));
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    var last_axis = 0u;

    while (t < t_max) {
        // Get the next voxel boundary
        let next = voxel + boundary;

        // Get minimum distance till next voxel boundaries
        let d = abs((vec3<f32>(next) * DIM_INV - position) * delta);

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = max(d[axis], 1E-5);

        if (visit(voxel, origin - 0.5, direction, position - 0.5, increment, t + increment, last_axis)) {
            break;
        }

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel[axis] = next[axis] + boundary[axis] - 1;
        last_axis = axis;
    }

    return result(origin - 0.5, direction);
}

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}

fn minimum4(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}
