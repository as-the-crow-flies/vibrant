use block;
use color;
use gaussian;
use math;
use occlusion;
use uniforms;
use vertex;

@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
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

// The shared octahedral radiance cascade (same probe grid the volume tracer
// samples). No HDRI group: the cascade already integrates the environment.
@group(2) @binding(0) var RADIANCE: texture_3d<f32>;
@group(2) @binding(1) var GAUSSIAN: texture_3d<f32>;
@group(2) @binding(2) var VMM_SAMPLER: sampler;
@group(2) @binding(3) var EXTINCTION: texture_3d<f32>;
@group(2) @binding(4) var LINE_EXTINCTION: texture_3d<f32>;
@group(2) @binding(5) var EXTINCTION_SAMPLER: sampler;
@group(2) @binding(6) var<uniform> VOLUME_TRANSFORM: mat4x4<f32>;
@group(2) @binding(7) var<uniform> CASCADE_OPTS: CascadeOpts;

@group(3) @binding(0) var<uniform> TRACTOGRAPHY: Tractography;

var<private> DIM: f32;
var<private> DIM_INV: f32;
var<private> RADIUS: f32;
// Ray direction (normalized, volume-[0,1] space) and near→far segment length of
// the current pixel's ray, set by `fragment` / `raymarch` for `shade` and
// `hit_fraction`.
var<private> DIRECTION: vec3<f32>;
var<private> RAY_LENGTH: f32;

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

struct FragmentOut {
    // Un-tone-mapped radiance; opaque (alpha forced to 1).
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

    // View space: an imprecise `w` only slides a point along its own ray.
    let near = unproject(vec3<f32>(fragment.uv, 0.0));
    let far = unproject(vec3<f32>(fragment.uv, 1.0));

    let camera = TRANSFORM * ENVIRONMENT.camera.transform;
    let origin = (camera * vec4<f32>(near, 1.0)).xyz;
    let segment = (camera * vec4<f32>(far - near, 0.0)).xyz;

    RAY_LENGTH = length(segment);

    let direction = segment / RAY_LENGTH;

    let color = raymarch(origin, direction);

    var out: FragmentOut;
    out.depth = hit_fraction();
    out.color = select(vec4<f32>(0.0), vec4<f32>(color.rgb, color.a), color.a > 0.0);
    return out;
}

fn raymarch(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    DIRECTION = direction;

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

fn shade(
    v0: Vertex,
    v1: Vertex,
    v0s: f32,
    v1s: f32,
    radius: f32,
    position: vec3<f32>,
    settings: LineSettings,
    index: u32
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

    // `position` is centered volume space ([-0.5, 0.5]); the probe grid is [0, 1].
    let probe_p = clamp(position + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    let view = -DIRECTION;

    let roughness = clamp(TRACTOGRAPHY.line_roughness, 0.04, 1.0);
    let f0 = vec3<f32>(0.16 * TRACTOGRAPHY.line_specular * TRACTOGRAPHY.line_specular);

    var visibility = vec2<f32>(1.0);
    if (CASCADE_OPTS.include_lines != 0u && TRACTOGRAPHY.line_occlusion > 0.0 && TRACTOGRAPHY.line_translucency < 1.0) {
        visibility = line_visibility(position, normal_smooth, index, !is_start, !is_end);
    }

    let radiance = sample_outgoing_radiance_surface(
        RADIANCE, GAUSSIAN, VMM_SAMPLER,
        probe_p, normal_smooth, view, albedo, roughness, f0,
        TRACTOGRAPHY.ambient_light, TRACTOGRAPHY.direct_light,
        near_transmission(probe_p, normal_smooth) * visibility.x,
        near_transmission(probe_p, -normal_smooth) * visibility.y,
        CASCADE_OPTS.vmm_size
    );

    // Combined: opaque. Overlay (render_mode == 1): honor per-bundle opacity.
    let alpha = select(
        1.0,
        TRACTOGRAPHY.alpha * mix(v0.alpha, v1.alpha, height),
        TRACTOGRAPHY.render_mode == 1u
    );

    return vec4<f32>(radiance, alpha);
}

// Cascade 0 starts tracing one probe cell out; cover that gap with a single
// cone sample half a cell along the normal, footprint one cell.
fn near_transmission(p: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    if (TRACTOGRAPHY.line_occlusion <= 0.0) { return vec3<f32>(1.0); }

    let probes = max(floor(vec3<f32>(textureDimensions(GAUSSIAN)) / vec3<f32>(4.0, 4.0, 2.0)), vec3<f32>(1.0));
    let uv = clamp(p + 0.5 * normal / probes, vec3<f32>(0.0), vec3<f32>(1.0));
    let cell = 1.0 / (length(VOLUME_TRANSFORM[0].xyz) * probes.x); // mm

    var depth = vec3<f32>(0.0);
    if (CASCADE_OPTS.include_volume != 0u) {
        let mip = log2(f32(textureDimensions(EXTINCTION).x) / probes.x);
        depth += cell * unpack_rgb(textureSampleLevel(EXTINCTION, EXTINCTION_SAMPLER, uv, mip));
    }

    return exp(-TRACTOGRAPHY.line_occlusion * depth);
}

// Line contact occlusion (front, back) from the occupancy field: a few taps out
// from the surface (the back ones from the far side of the tube), each with the
// hit streamline's own coverage removed so lines don't shadow themselves.
const NEAR_TAPS: u32 = 3u;

// Coverage clamps matching the occupancy encoding: per-segment 1 - e^-4,
// per-voxel 1 - e^-8.
const SEGMENT_COVERAGE_MAX: f32 = 0.98168436;
const VOXEL_COVERAGE_MAX: f32 = 0.99966454;

// The trilinear lookup widens the voxelized one-voxel coverage ramp to ~1.75.
const OWN_RAMP: f32 = 1.75;

fn line_visibility(p: vec3<f32>, normal: vec3<f32>, index: u32, has_prev: bool, has_next: bool) -> vec2<f32> {
    let surface = (p + 0.5) * DIM;
    let radius = TRACTOGRAPHY.radius;

    var depth = vec2<f32>(0.0);

    for (var k = 0u; k < NEAR_TAPS; k++) {
        let t = f32(k) + 0.5;
        depth.x += other_depth(surface + normal * t, index, has_prev, has_next);
        depth.y += other_depth(surface - normal * (2.0 * radius + t), index, has_prev, has_next);
    }

    // Same extinction as the deposit, per voxel step: (1 - t) depth / (2 min(r, h)).
    let scale = 0.5 * (1.0 - TRACTOGRAPHY.line_translucency) / min(radius, max(1.0, TRACTOGRAPHY.smoothing));

    return exp(-TRACTOGRAPHY.line_occlusion * scale * depth);
}

// Occupancy depth at `q` (voxels) from everything but the hit streamline:
// -ln(1 - f) + ln(1 - own), with depths additive.
fn other_depth(q: vec3<f32>, index: u32, has_prev: bool, has_next: bool) -> f32 {
    let fraction = textureSampleLevel(DENSITY, SAMPLER, q * DIM_INV, 0.0).x;

    var empty = 1.0 - own_coverage(q, index);
    if (has_prev) { empty *= 1.0 - own_coverage(q, index - 1u); }
    if (has_next) { empty *= 1.0 - own_coverage(q, index + 1u); }

    let own = min(1.0 - empty, VOXEL_COVERAGE_MAX);

    return max(log(1.0 - own) - log(1.0 - min(fraction, VOXEL_COVERAGE_MAX)), 0.0);
}

// The voxelization's coverage of segment `index` at `q`, on the widened ramp.
fn own_coverage(q: vec3<f32>, index: u32) -> f32 {
    let v0 = unpack_vertex_scale(LINE_VERTEX[index + 0], DIM);
    let v1 = unpack_vertex_scale(LINE_VERTEX[index + 1], DIM);

    let radius = TRACTOGRAPHY.radius;
    let radius_clamp = max(TRACTOGRAPHY.smoothing, radius);
    let radius_ratio = pow(radius / radius_clamp, 2.0);

    let delta = v1.xyz - v0.xyz;
    let qv0 = q - v0.xyz;
    let qv1 = q - v1.xyz;

    let height = saturate(dot(qv0, delta) / dot(delta, delta));

    let sdf = max(length(qv0 - delta * height) - radius_clamp, max(-dot(qv0, v0.clip), dot(qv1, v1.clip)));

    let coverage = radius_ratio * mix(v0.alpha, v1.alpha, height) * saturate(0.5 - sdf / OWN_RAMP);

    return min(coverage, SEGMENT_COVERAGE_MAX);
}
