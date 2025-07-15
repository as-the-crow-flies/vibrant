@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(1) @binding(0) var<storage> OFFSET: array<u32>;
@group(1) @binding(2) var<storage> INDEX: array<u32>;

@group(2) @binding(0) var OCCUPANCY: texture_3d<f32>;

@group(3) @binding(0) var COUNT: texture_3d<u32>;

@group(4) @binding(0) var DENSITY: texture_3d<f32>;
@group(4) @binding(1) var SAMPLER: sampler;

@group(5) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(6) @binding(0) var COLOR: texture_storage_2d<bgra8unorm, write>;

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    if (any(pixel.xy >= ENVIRONMENT.surface)) { return; }

    let result = compute(pixel.xy);

    textureStore(COLOR, vec2<u32>(pixel.x, ENVIRONMENT.surface.y - pixel.y), result);
}

var<private> DIM: f32;
var<private> DIM_INV: f32;
var<private> DIR_INV: f32;
var<private> RADIUS: f32;
var<private> ALPHA: f32;

fn compute(pixel: vec2<u32>) -> vec4<f32> {
    DIM = f32(textureDimensions(DENSITY).x);
    DIM_INV = 1.0 / DIM;
    RADIUS = ENVIRONMENT.settings.streamline_radius;
    ALPHA = ENVIRONMENT.settings.alpha;

    let uv = vec2<f32>(pixel) / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0;

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));
    let direction = normalize(far - near);

    let DIR_INV = 1.0 / direction;

    return raymarch(near + 0.5, direction);
}

fn raymarch(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    let delta = select(1.0 / direction, vec3<f32>(1E10), abs(direction) < vec3<f32>(1E-5));
    let boundary = select(vec3<u32>(0), vec3<u32>(1), direction >= vec3<f32>(0.0));

    let tMinBounds = (vec3<f32>(0.0) - origin) * delta;
    let tMaxBounds = (vec3<f32>(1.0) - origin) * delta;

    let tEnter = maximum(min(tMinBounds, tMaxBounds)) + 1E-5;
    let tExit = minimum(max(tMinBounds, tMaxBounds)) - 1E-5;

    if (tEnter >= tExit || tExit < 0.0) { return vec4<f32>(0.0); }

    var t = max(tEnter, 0.0);
    var mip = textureNumLevels(OCCUPANCY) - 1;
    var position = origin + direction * t;
    var voxel = vec3<u32>(floor(position * DIM));

    while (t < tExit) {
        // Get the current voxel at the current mip level
        let voxel_at_mip = voxel >> vec3<u32>(mip);

        // Traverse down level if mip is occupied
        if (textureLoad(OCCUPANCY, voxel_at_mip, i32(mip)).x > 0.0) {
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

    var color = vec4<f32>(0.0);

    while (t < tExit) {
        // Get the next voxel boundary
        let next = voxel + boundary;

        // Get minimum distance till next voxel boundaries
        let d = abs((vec3<f32>(next) * DIM_INV - position) * delta);

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = max(d[axis], 1E-5);

        // Accumulate Color
        color += (1.0 - color.a) * intersect(voxel, position * DIM, direction, increment * DIM);
        if (color.a > 0.95) { return color; }

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    return color;
}

fn intersect(voxel: vec3<u32>, origin: vec3<f32>, direction: vec3<f32>, increment: f32) -> vec4<f32> {
    return gather(voxel, origin, direction, increment);
}

fn gather(voxel: vec3<u32>, origin: vec3<f32>, direction: vec3<f32>, increment: f32) -> vec4<f32> {
    let count = textureLoad(COUNT, voxel, 0).x;
    let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;

    let increment_inv = 1.0 / increment;
    let increment_half = 0.5 * increment;
    let midpoint = origin + direction * increment_half;
    let r = RADIUS + increment_half;

    var closest = U32_MAX;

    for (var i = 0u; i < count; i++) {
        let index = INDEX[offset + i];

        let v0 = LINE_VERTEX[index + 0];
        let v1 = LINE_VERTEX[index + 1];

        if (line_distance(midpoint, v0.xyz, v1.xyz) > r) { continue; }

        let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);

        if (hit < 0 || hit >= increment) { continue; }

        let candidate = (u32((hit * increment_inv) * U16_MAX_f32) << 16) | i;

        closest = min(closest, candidate);
    }

    if (closest != U32_MAX) {
        let hit = f32(closest >> 16) * U16_MAX_INV * increment;
        let index = INDEX[offset + (closest & U16_MAX)];

        let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
        let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

        let position = origin + hit * direction;

        let delta = v1.xyz - v0.xyz;
        let pa = position - v0.xyz;
        let height = saturate(dot(pa, delta) / dot(delta, delta));
        let normal =  (pa - height * delta) / RADIUS;

        let delta_norm = normalize(delta);
        let tangent = normalize(mix(
            select(v0.clip, delta_norm, all(v0.clip == vec3<f32>())),
            select(v1.clip, delta_norm, all(v1.clip == vec3<f32>())),
            height
        ));

        let light = ENVIRONMENT.light;

        let factor = mix(1.0, lambert(normal, light), ENVIRONMENT.settings.direct_light);

        return vec4<f32>(factor * abs(tangent), 1.0);
    }

    return vec4<f32>(0.0);
}

fn clip(position: vec3<f32>, v0: vec4<f32>, v1: vec4<f32>) -> bool {
    let n0 = unpack4x8snorm(bitcast<u32>(v0.w));
    let n1 = unpack4x8snorm(bitcast<u32>(v1.w));

    let clipped = dot(position - v0.xyz, n0.xyz) < 0.0 || dot(v1.xyz - position, n1.xyz) < 0.0;

    return clipped;
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

fn point_to_plane(point: vec3<f32>, plane_point: vec3<f32>, plane_normal: vec3<f32>) -> vec3<f32> {
    return point - dot(point - plane_point, plane_normal) * plane_normal;
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

    return 1E6;
}

fn line_distance(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = clamp(dot(pa,ba) / dot(ba,ba), 0.0, 1.0);
  return length(pa - ba*h);
}
