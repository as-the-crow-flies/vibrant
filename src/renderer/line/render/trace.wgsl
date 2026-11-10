use block;
use color;
use gaussian;
use intersect;
use math;
use noise;
use occlusion;
use uniforms;
use vertex;

@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(4) var<storage> LINE_MATERIAL: array<u32>;
@group(0) @binding(5) var<storage> LINE_SETTINGS: array<LineSettings>;
@group(0) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(0) @binding(7) var<storage> LINE_SCALAR: array<f32>;
@group(0) @binding(8) var COLORMAP: texture_3d<f32>;
@group(0) @binding(9) var DENSITY: texture_3d<f32>;
@group(0) @binding(10) var SAMPLER: sampler;
@group(0) @binding(11) var COUNT: texture_3d<u32>;
// 13..16 intentionally free (removed line occlusion volumes).
@group(0) @binding(17) var<storage> OFFSET: array<u32>;
@group(0) @binding(18) var<storage> INDEX: array<u32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(1) var BLUE_NOISE: texture_3d<f32>;

// The shared octahedral radiance cascade (same probe grid the volume tracer
// samples). No HDRI group: the cascade already integrates the environment.
@group(2) @binding(0) var RADIANCE: texture_3d<f32>;
@group(2) @binding(1) var GAUSSIAN: texture_3d<f32>;
@group(2) @binding(2) var VMM_SAMPLER: sampler;
@group(2) @binding(7) var<uniform> CASCADE_OPTS: CascadeOpts;

@group(3) @binding(0) var<uniform> TRACTOGRAPHY: Tractography;

var<private> DIM: f32;
var<private> DIM_INV: f32;
var<private> RADIUS: f32;
// The ray's view direction in world space, where the lobes live.
var<private> VIEW: vec3<f32>;
// Per-pixel [0, 1) for the stochastic opacity test, see `is_kept`.
var<private> COIN: f32;
// Per-pixel probe lookup offset in probe cells, see `shade`.
var<private> LOOKUP: vec3<f32>;

struct Hit {
    distance: f32,
    index: u32,
    position: vec3<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

struct FragmentOut {
    // Un-tone-mapped radiance of the nearest kept hit; transparent on a miss.
    @location(0) color: vec4<f32>,
    // Near→far fraction of the first hit; 1.0 on miss. The volume tracer clamps
    // its far `t` to this.
    @location(1) depth: f32,
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
fn fragment(fragment: Fragment) -> FragmentOut {
    DIM = f32(TRACTOGRAPHY.volume);
    DIM_INV = 1.0 / DIM;
    RADIUS = TRACTOGRAPHY.radius * DIM_INV;
    COIN = noise(vec2<u32>(fragment.position.xy), 2u).x;
    LOOKUP = TRACTOGRAPHY.lookup_jitter * tent(noise(vec2<u32>(fragment.position.xy), 3u).xyz);

    // View space: an imprecise `w` only slides a point along its own ray.
    let near = unproject(vec3<f32>(fragment.uv, 0.0));
    let far = unproject(vec3<f32>(fragment.uv, 1.0));

    let camera = TRANSFORM * ENVIRONMENT.camera.transform;
    let origin = (camera * vec4<f32>(near, 1.0)).xyz;
    let segment = (camera * vec4<f32>(far - near, 0.0)).xyz;

    let ray_length = length(segment);
    VIEW = -normalize((ENVIRONMENT.camera.transform * vec4<f32>(far - near, 0.0)).xyz);

    let hit = raymarch(origin, segment / ray_length);

    if (hit.index == U32_MAX) { return FragmentOut(vec4<f32>(0.0), 1.0); }

    let depth = clamp(hit.distance / max(ray_length, 1e-6), 0.0, 1.0);
    return FragmentOut(vec4<f32>(shade(hit), 1.0), depth);
}

// Stochastic opacity: a fibre is kept with probability `alpha`, so the nearest
// kept hit averages to sorted front-to-back compositing. One coin per fibre
// (not per segment) avoids double counting where capsules overlap; rotating
// the pixel's noise by the fibre's hash (packed in `w`) keeps its dither blue.
fn is_kept(w: f32) -> bool {
    if (TRACTOGRAPHY.alpha >= 1.0) { return true; }

    let offset = (f32(unpack_hash(w)) + 0.5) / 256.0;
    return fract(COIN + offset) < TRACTOGRAPHY.alpha;
}

fn raymarch(origin: vec3<f32>, direction: vec3<f32>) -> Hit {
    let delta = select(1.0 / direction, vec3<f32>(1E10), abs(direction) < vec3<f32>(1E-5));
    let boundary = select(vec3<u32>(0), vec3<u32>(1), direction >= vec3<f32>(0.0));

    let tMinBounds = (vec3<f32>(0.0) - origin) * delta;
    let tMaxBounds = (vec3<f32>(1.0) - origin) * delta;

    let t_min = maximum(min(tMinBounds, tMaxBounds)) + 1E-5;
    let t_max = minimum(max(tMinBounds, tMaxBounds)) - 1E-5;

    if (t_min >= t_max || t_max < 0.0) { return Hit(0.0, U32_MAX, vec3<f32>()); }

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

    while (t < t_max) {
        // Get the next voxel boundary
        let next = voxel + boundary;

        // Get minimum distance till next voxel boundaries
        let d = abs((vec3<f32>(next) * DIM_INV - position) * delta);

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = max(d[axis], 1E-5);

        let hit = visit(voxel, position - 0.5, direction, increment, t + increment);
        if (hit.index != U32_MAX) { return hit; }

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    return Hit(0.0, U32_MAX, vec3<f32>());
}

// Nearest kept hit inside `voxel`. Intersects from the voxel entry, not the ray
// origin: the origin can be far away (narrow fov, orthographic), which wrecks
// the capsule quadratic.
fn visit(voxel: vec3<u32>, position: vec3<f32>, direction: vec3<f32>, increment: f32, distance: f32) -> Hit {
    var hit = Hit(distance, U32_MAX, vec3<f32>());

    let count = textureLoad(COUNT, voxel, 0).x;
    if (count == 0) { return hit; }

    let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;
    let base = distance - increment;

    for (var i = 0u; i < count; i++) {
        let index = INDEX[offset + i];

        let v0 = LINE_VERTEX[index];

        let local = capsule_intersection(position, direction, v0.xyz, LINE_VERTEX[index + 1].xyz, RADIUS);
        let intersection = base + local;

        if (intersection < hit.distance && is_kept(v0.w)) {
            hit = Hit(intersection, index, position + direction * local);
        }
    }

    return hit;
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

fn shade(hit: Hit) -> vec3<f32> {
    let v0 = unpack_vertex(LINE_VERTEX[hit.index + 0]);
    let v1 = unpack_vertex(LINE_VERTEX[hit.index + 1]);

    let v0s = LINE_SCALAR[hit.index + 0];
    let v1s = LINE_SCALAR[hit.index + 1];

    let settings = LINE_SETTINGS[LINE_MATERIAL[hit.index]];
    let position = hit.position;

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

    let normal = normalize((pa - height * delta) / RADIUS);

    let use_original_normal = (is_start && height == 0.0) || (is_end && height == 1.0);
    let normal_smooth = select(orthonormalize(normal, tangent), normal, use_original_normal);

    // Albedo: tangent RGB / fixed bundle color / scalar colormap. The old
    // Lambert + occlusion term is gone -- lighting is the shared VMM Disney
    // BRDF below.
    let tangent_color = tangent2rgb(abs(tangent));
    let line_color = unpack4x8unorm(settings.color);

    let scalar_sample = u32(mix(v0s, v1s, height) * 255.0);
    let scalar_color = textureLoad(COLORMAP, vec3<u32>(scalar_sample, settings.colormap, 0u), 0).rgb;

    let has_scalar_color = line_color.a == 1.0;
    let has_line_color = !has_scalar_color && line_color.a > 0.4;
    let has_tangent_color = !has_line_color && !has_scalar_color;

    let albedo =
        f32(has_line_color) * line_color.rgb +
        f32(has_scalar_color) * scalar_color +
        f32(has_tangent_color) * tangent_color;

    // Accumulated, the jittered lookup turns the blocky trilinear probe
    // interpolation into a smooth B-spline (as in the volume tracer).
    // `position` is centered volume space ([-0.5, 0.5]); the probe grid is [0, 1].
    let probe_p = clamp(position + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    // The tube normal lives in volume space; normals map to world by the
    // transpose of the world → volume transform.
    let world_normal = normalize(transpose(linear_part(TRANSFORM)) * normal_smooth);

    let roughness = clamp(TRACTOGRAPHY.line_roughness, 0.04, 1.0);
    let f0 = vec3<f32>(0.16 * TRACTOGRAPHY.line_specular * TRACTOGRAPHY.line_specular);

    let radiance = sample_outgoing_radiance_surface(
        RADIANCE, GAUSSIAN, VMM_SAMPLER,
        probe_p, LOOKUP, world_normal, VIEW, albedo, roughness, f0,
        TRACTOGRAPHY.ambient_light, TRACTOGRAPHY.direct_light,
        CASCADE_OPTS.vmm_size
    );

    return radiance;
}
