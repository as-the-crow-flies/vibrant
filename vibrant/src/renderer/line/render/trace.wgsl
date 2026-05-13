@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(0) @binding(2) var COUNT: texture_3d<u32>;
@group(0) @binding(4) var OCCLUSION_AMBIENT: texture_3d<f32>;
@group(0) @binding(6) var OCCLUSION_DIRECTIONAL: texture_3d<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(2) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(2) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(2) @binding(4) var<storage> LINE_MATERIAL: array<u32>;
@group(2) @binding(5) var<storage> LINE_SETTINGS: array<LineSettings>;
@group(2) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(2) @binding(7) var<storage> LINE_SCALAR: array<f32>;
@group(2) @binding(8) var COLORMAP: texture_2d<f32>;

@group(3) @binding(0) var<storage> OFFSET: array<u32>;
@group(3) @binding(2) var<storage> INDEX: array<u32>;

var<private> DIM: f32;
var<private> DIM_INV: f32;
var<private> DIR_INV: f32;
var<private> RADIUS: f32;
var<private> ALPHA: f32;

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Fragment {
    let uv = vec2<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2))
    );

    return Fragment(vec4<f32>(uv, 0.0, 1.0), uv);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    DIM = f32(ENVIRONMENT.volume);
    DIM_INV = 1.0 / DIM;
    RADIUS = ENVIRONMENT.settings.radius * DIM_INV;
    ALPHA = ENVIRONMENT.settings.alpha;

    let near = unproject(vec3<f32>(fragment.uv, 0.0));
    let far = unproject(vec3<f32>(fragment.uv, 1.0));

    let origin = near;
    let direction = normalize(far - near);

    let result = raymarch(origin, direction);

    if (result.a > 0.0) { return result; }
    else { return vec4<f32>(0.0); }
    // else { return background(origin, direction); }
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
    let t = TRANSFORM * ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}

fn background(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    let hit = plane_intersection(origin, direction, vec4<f32>(0.0, 1.0, 0.0, -ENVIRONMENT.settings.plane));
    if (hit <= 0.0) { return vec4<f32>(1.0); }

    let dim = f32(textureDimensions(DENSITY).x);
    let one_over_dim = 1.0 / dim;

    let position = origin + direction * hit;

    var directional_occlusion = 0.0;

    for (var distance = 1.0; distance < 2.0 * dim; distance += 1.0) {
        let sample = (position + ENVIRONMENT.light * distance * one_over_dim);

        directional_occlusion += (1.0 - directional_occlusion) * textureSampleLevel(DENSITY, SAMPLER, sample, 0.0).x;

        if (directional_occlusion > 0.99) { break; }
    }

    var ambient_occlusion = 0.0;

    for (var i=0u; i<12; i++) {
        var icosahedron = ICOSAHEDRON[i];

        var occlusion = 0.0;

        for (var distance = 1.0; distance < dim * 2.0; distance *= 2.0) {
            let sample = (position + icosahedron * distance * one_over_dim);
            let level = log2(TAN_CONE_ANGLE * distance);

            occlusion += (1.0 - occlusion) * textureSampleLevel(DENSITY, SAMPLER, sample, level).x;

            if (occlusion > 0.99) { break; }
        }

        ambient_occlusion += occlusion;
    }

    ambient_occlusion = clamp(ambient_occlusion / 6.0, 0.0, 1.0);

    let ao = 1.0 - ambient_occlusion;
    let shadow = 1.0 - ENVIRONMENT.settings.direct_light * directional_occlusion;
    let occlusion = 1.0 - (1.0 - shadow) * (1.0 - ao);

    return vec4<f32>(vec3<f32>(occlusion), 1.0);
}

fn shade(
    v0: Vertex,
    v1: Vertex,
    v0s: f32,
    v1s: f32,
    radius: f32,
    position: vec3<f32>,
    settings: LineSettings
) -> vec4<f32> {
    let delta = v1.xyz - v0.xyz;
    let pa = position - v0.xyz;
    let height = saturate(dot(pa, delta) / dot(delta, delta));

    let is_start = all(v0.clip == vec3<f32>());
    let is_end = all(v1.clip == vec3<f32>());

    let delta_norm = normalize(delta);
    let tangent = normalize(mix(
        select(v0.clip, delta_norm, is_start),
        select(v1.clip, delta_norm, is_end),
        height
    ));

    let normal = normalize((pa - height * delta) / radius);

    let use_original_normal = (is_start && height == 0.0) || (is_end && height == 1.0);
    let normal_smooth = select(orthonormalize(normal, tangent), normal, use_original_normal);
    let diffuse = lambert(normal_smooth, ENVIRONMENT.light);

    let ambient = 1.0 - textureSampleLevel(OCCLUSION_AMBIENT, SAMPLER, position + 0.5, 0.0).x;
    let directional = 1.0 - textureSampleLevel(OCCLUSION_DIRECTIONAL, SAMPLER, position + 0.5, 0.0).x;

    let ao = ambient * ENVIRONMENT.settings.ambient_light;
    let shadow = diffuse * ENVIRONMENT.settings.direct_light * directional;
    let factor = 1.0 - (1.0 - shadow) * (1.0 - ao);

    let color = unpack4x8unorm(settings.color);

    let tangent_color = tangent2rgb(abs(tangent));
    let line_color = unpack4x8unorm(settings.color);

    let scalar_sample = u32(mix(v0s, v1s, height) * 255.0);
    let scalar_color = textureLoad(COLORMAP, vec2<u32>(scalar_sample, settings.colormap), 0).rgb;

    let has_scalar_color = line_color.a == 1.0;
    let has_line_color = !has_scalar_color && line_color.a > 0.4;
    let has_tangent_color = !has_line_color && !has_scalar_color;

    let rgb = factor * (
        f32(has_line_color) * line_color.rgb +
        f32(has_scalar_color) * scalar_color +
        f32(has_tangent_color) * tangent_color
    );

    let alpha = ENVIRONMENT.settings.alpha * mix(v0.alpha, v1.alpha, height);

    return vec4<f32>(rgb, alpha);
}
