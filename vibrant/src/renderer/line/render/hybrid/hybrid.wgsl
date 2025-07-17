@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(1) @binding(0) var<storage> OFFSET: array<u32>;
@group(1) @binding(2) var<storage> INDEX: array<u32>;

@group(2) @binding(0) var OCCUPANCY: texture_3d<f32>;
@group(3) @binding(0) var COUNT: texture_3d<u32>;

@group(4) @binding(0) var DENSITY: texture_3d<f32>;
@group(4) @binding(1) var DENSITY_SAMPLER: sampler;

@group(5) @binding(0) var AMBIENT_OCCLUSION: texture_3d<f32>;
@group(5) @binding(1) var AMBIENT_OCCLUSION_SAMPLER: sampler;

@group(6) @binding(0) var DIRECTIONAL_OCCLUSION: texture_3d<f32>;
@group(6) @binding(1) var DIRECTIONAL_OCCLUSION_SAMPLER: sampler;

@group(7) @binding(0) var<uniform> ENVIRONMENT: Environment;

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
    DIM = f32(textureDimensions(DENSITY).x);
    DIM_INV = 1.0 / DIM;
    RADIUS = ENVIRONMENT.settings.streamline_radius;
    ALPHA = ENVIRONMENT.settings.alpha;

    let uv = vec2<f32>(1.0, -1.0) * (pixel.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);

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
        color += (1.0 - color.a) * visit(voxel, position * DIM, direction, increment * DIM);
        if (color.a > 0.95) { return color; }

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    return color;
}

fn visit(voxel: vec3<u32>, origin: vec3<f32>, direction: vec3<f32>, increment: f32) -> vec4<f32> {
    let count = textureLoad(COUNT, voxel, 0).x;
    let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;

    let increment_inv = 1.0 / increment;


    var hit_count = 0u;
    var hits = array<u32, 32>();

    for (var i = 0u; i < count && hit_count < 32; i++) {
        let index = INDEX[offset + i];

        let hit = hittest(index, origin, direction, increment);

        if (hit < 0.0) { continue; }

        let candidate = (u32((hit * increment_inv) * U16_MAX_f32) << 16) | i;

        hits[hit_count] = candidate;
        hit_count++;
    }

    if (hit_count > 0) {
        sort(&hits, hit_count);

        var color = vec4<f32>(0.0);

        for (var i=0u; i<hit_count; i++) {
            let item = hits[i];
            let index = INDEX[offset + (item & U16_MAX)];

            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            let t_min = f32(item >> 16) * U16_MAX_INV * increment;

            let position = origin + t_min * direction;

            let c = shade(v0, v1, position);

            color += (1.0 - color.a) * vec4<f32>(c.rgb * c.a, c.a);

            if (color.a > 0.99) { break; }
        }

        return color;
    }

    return vec4<f32>(0.0);
}

fn hittest(index: u32, origin: vec3<f32>, direction: vec3<f32>, increment: f32) -> f32 {
    let v0 = LINE_VERTEX[index + 0];
    let v1 = LINE_VERTEX[index + 1];

    let increment_half = 0.5 * increment;
    let midpoint = origin + direction * increment_half;
    let r = RADIUS + increment_half;

    if (line_distance(midpoint, v0.xyz, v1.xyz) > r) { return -1.0; }

    let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);

    return select(hit, -1.0, hit >= increment);
}

fn shade(v0: Vertex, v1: Vertex, position: vec3<f32>) -> vec4<f32> {
    let clipped = dot(position - v0.xyz, v0.clip) < 0.0 || dot(v1.xyz - position, v1.clip) < 0.0;

    if (clipped) { return vec4<f32>(0.0); }

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

    let normal = (pa - height * delta) / RADIUS;

    let use_original_normal = (is_start && height == 0.0) || (is_end && height == 1.0);
    let normal_smooth = select(orthonormalize(normal, tangent), normal, use_original_normal);
    let diffuse = lambert(normal_smooth, ENVIRONMENT.light);

    let sample = position * DIM_INV;
    let ambient = 1.0 - textureSampleLevel(AMBIENT_OCCLUSION, AMBIENT_OCCLUSION_SAMPLER, sample, 0.0).x;
    let directional = 1.0 - textureSampleLevel(DIRECTIONAL_OCCLUSION, DIRECTIONAL_OCCLUSION_SAMPLER, sample, 0.0).x;

    let factor = mix(ambient, diffuse * directional, ENVIRONMENT.settings.direct_light);

    let rgb = factor * mix(vec3<f32>(1.0), abs(tangent), ENVIRONMENT.settings.tangent_color);
    let a = ENVIRONMENT.settings.alpha * mix(v0.alpha, v1.alpha, height);

    return vec4<f32>(rgb, a);
}

fn sort(data: ptr<function, array<u32, 32>>, count: u32) {
    for (var i: u32 = 1u; i < count; i = i + 1u) {
        let key = (*data)[i];
        var j: i32 = i32(i) - 1;

        while (j >= 0 && (*data)[u32(j)] > key) {
            (*data)[u32(j + 1)] = (*data)[u32(j)];
            j = j - 1;
        }

        (*data)[u32(j + 1)] = key;
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

fn orthonormalize(normal: vec3<f32>, tangent: vec3<f32>) -> vec3<f32> {
    return normalize(normal - dot(normal, tangent) * tangent);
}

// https://iquilezles.org/articles/intersectors
// https://www.shadertoy.com/view/Xt3SzX
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

    if (h>=0.0) {
        let t = (-b - sqrt(h)) / a;
        let y = baoa + t*bard;

        // body
        if(y > 0.0 && y < baba) { return t; }

        // caps
        let oc = select(ro - pb, oa, y <= 0.0);

        b = dot(rd, oc);
        c = dot(oc, oc) - r*r;
        h = b*b - c;
        if (h > 0.0) { return -b - sqrt(h); }
    }

    return 1E6;
}

fn capsule_intersection_back(ro: vec3<f32>, rd: vec3<f32>, pa: vec3<f32>, pb: vec3<f32>, r: f32) -> f32
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

    if (h>=0.0) {
        let t = (-b + sqrt(h)) / a;
        let y = baoa + t*bard;

        // body
        if(y > 0.0 && y < baba) { return t; }

        // caps
        let oc = select(ro - pb, oa, y <= 0.0);

        b = dot(rd, oc);
        c = dot(oc, oc) - r*r;
        h = b*b - c;
        if (h > 0.0) { return -b + sqrt(h); }
    }

    return 1E6;
}

fn line_distance(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = clamp(dot(pa,ba) / dot(ba,ba), 0.0, 1.0);
  return length(pa - ba*h);
}
