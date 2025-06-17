@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(1) @binding(0) var<storage> OFFSET: array<u32>;
@group(1) @binding(2) var<storage> INDEX: array<u32>;

@group(2) @binding(0) var OCCUPANCY: texture_3d<f32>;

@group(3) @binding(0) var COUNT: texture_3d<u32>;

@group(4) @binding(0) var DENSITY: texture_3d<f32>;
@group(4) @binding(1) var SAMPLER: sampler;

@group(5) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(6) @binding(0) var COLOR: texture_storage_2d<bgra8unorm, write>;

@compute
@workgroup_size(64, 1)
fn main(@builtin(workgroup_id) tile: vec3<u32>, @builtin(local_invocation_index) local: u32) {
    let pixel = tile.xy * 8 + vec2<u32>(local & 7, local >> 3);

    if (any(pixel >= ENVIRONMENT.surface)) { return; }

    let result = compute(pixel);

    textureStore(COLOR, vec2<u32>(pixel.x, ENVIRONMENT.surface.y - pixel.y), result);
}

const LOCAL_SORT_SIZE: u32 = 32;
var<private> HIT_DISTANCE: array<f32, LOCAL_SORT_SIZE>;
var<private> HIT_INDEX: array<u32, LOCAL_SORT_SIZE>;
var<private> HIT_COUNT: u32 = 0;

var<private> DIM: f32;
var<private> DIM_INV: f32;
var<private> RADIUS: f32;
var<private> ALPHA: f32;

fn compute(pixel: vec2<u32>) -> vec4<f32> {
    DIM = f32(textureDimensions(OCCUPANCY).x);
    DIM_INV = 1.0 / DIM;
    RADIUS = ENVIRONMENT.settings.streamline_radius * DIM_INV;
    ALPHA = ENVIRONMENT.settings.alpha;

    let uv = vec2<f32>(pixel) / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0;

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));
    let direction = normalize(far - near);

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
        color += (1.0 - color.a) * intersect(voxel, origin - 0.5, direction, tExit);
        if (color.a > 0.95) { return color; }

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    return color;
}

fn intersect(voxel: vec3<u32>, origin: vec3<f32>, direction: vec3<f32>, distance: f32) -> vec4<f32> {
    gather(voxel, origin, direction);
    sort(&HIT_INDEX, &HIT_DISTANCE, HIT_COUNT);
    return blend(origin, direction);
}

fn gather(voxel: vec3<u32>, origin: vec3<f32>, direction: vec3<f32>) {
    HIT_COUNT = 0u;

    let count = textureLoad(COUNT, voxel, 0).x;
    let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;

    for (var i = 0u; i < count; i++) {
        let index = INDEX[offset + i];

        let v0 = TRACTOGRAM_VERTICES[index + 0];
        let v1 = TRACTOGRAM_VERTICES[index + 1];

        let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);

        if (hit >= 0.0 && within_voxel(origin + hit * direction, voxel, v0, v1)) {
            HIT_INDEX[HIT_COUNT] = index;
            HIT_DISTANCE[HIT_COUNT] = hit;

            HIT_COUNT++;
        }
    }
}

fn blend(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    var color = vec4<f32>(0.0);

    for (var i = 0u; i < HIT_COUNT; i++) {
        let hit = HIT_DISTANCE[i];
        let index = HIT_INDEX[i];

        let position = origin + hit * direction;

        let v0 = TRACTOGRAM_VERTICES[index + 0].xyz;
        let v1 = TRACTOGRAM_VERTICES[index + 1].xyz;

        let rgb = capsule_normal(position, v0, v1, RADIUS) * 0.5 + 0.5;

        color += (1.0 - color.a) * vec4<f32>(rgb * ALPHA, ALPHA);

        if (color.a > 0.95) { return color; }
    }

    return color;
}

fn within_voxel(position: vec3<f32>, voxel: vec3<u32>, v0: vec4<f32>, v1: vec4<f32>) -> bool {
    let not_in_voxel = any(vec3<u32>((position + 0.5) * DIM) != voxel);

    return !not_in_voxel;

    // if (not_in_voxel) {
    //     return false;
    // }

    // let n0 = unpack4x8snorm(bitcast<u32>(v0.w));
    // let n1 = unpack4x8snorm(bitcast<u32>(v1.w));

    // let clipped = dot(position - v0.xyz, n0.xyz) < 0.0 || dot(v1.xyz - position, n1.xyz) < 0.0;

    // return !clipped;
}

fn sort(
    index: ptr<private, array<u32, LOCAL_SORT_SIZE>>,
    distance: ptr<private, array<f32, LOCAL_SORT_SIZE>>,
    count: u32
) {
    for (var i: u32 = 1u; i < count; i++) {
        let key = (*distance)[i];
        let idx = (*index)[i];

        var j: i32 = i32(i) - 1;

        // Move elements of distance[0..i-1] that are greater than key
        while (j >= 0 && (*distance)[u32(j)] > key) {
            (*index)[u32(j + 1)] = (*index)[u32(j)];
            (*distance)[u32(j + 1)] = (*distance)[u32(j)];
            j--;
        }

        (*index)[u32(j + 1)] = idx;
        (*distance)[u32(j + 1)] = key;
    }
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

    return -1.0;
}

fn capsule_normal(pos: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> vec3<f32>
{
    let ba = b - a;
    let pa = pos - a;
    let h = saturate(dot(pa, ba) / dot(ba, ba));
    return (pa - h*ba) / r;
}
