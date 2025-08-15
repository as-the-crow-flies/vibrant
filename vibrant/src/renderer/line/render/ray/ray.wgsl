@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(1) @binding(0) var<storage> OFFSET: array<u32>;
@group(1) @binding(2) var<storage> INDEX: array<u32>;
@group(1) @binding(3) var CULLING: texture_3d<f32>;

@group(2) @binding(0) var DENSITY: texture_3d<f32>;
@group(2) @binding(1) var SAMPLER: sampler;
@group(2) @binding(2) var COUNT: texture_3d<u32>;
@group(2) @binding(4) var OCCLUSION_AMBIENT: texture_3d<f32>;
@group(2) @binding(6) var OCCLUSION_DIRECTIONAL: texture_3d<f32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const LOCAL_SORT_SIZE: u32 = 32;

var<private> DIM: f32;
var<private> DIM_INV: f32;
var<private> DIR_INV: f32;
var<private> RADIUS: f32;
var<private> ALPHA: f32;

struct Hit {
    distance: f32,
    index: u32
}

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
    RADIUS = ENVIRONMENT.settings.radius;
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

    let t_min = maximum(min(tMinBounds, tMaxBounds)) + 1E-5;
    let t_max = minimum(max(tMinBounds, tMaxBounds)) - 1E-5;

    if (t_min >= t_max || t_max < 0.0) { return vec4<f32>(0.0); }

    var t = max(t_min, 0.0);
    var mip = textureNumLevels(CULLING) - 1;
    var position = origin + direction * t;
    var voxel = vec3<u32>(floor(position * DIM));

    while (t < t_max) {
        // Get the current voxel at the current mip level
        let voxel_at_mip = voxel >> vec3<u32>(mip);

        // Traverse down level if mip is occupied
        if (textureLoad(CULLING, voxel_at_mip, i32(mip)).x > 0.0) {
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

    if (ENVIRONMENT.settings.alpha < 1.0) {
        var color = vec4<f32>(0.0);

        while (t < t_max) {
            // Get the next voxel boundary
            let next = voxel + boundary;

            // Get minimum distance till next voxel boundaries
            let d = abs((vec3<f32>(next) * DIM_INV - position) * delta);

            // Get axis of smallest distance
            let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

            // Get Increment
            let increment = max(d[axis], 1E-5);

            let count = textureLoad(COUNT, voxel, 0).x;
            let culled = textureLoad(CULLING, voxel, 0).x < 0.5;

            // Accumulate Color
            if (count > 0 && !culled) {
                color += (1.0 - color.a) * visit_transparency(count, voxel, position * DIM, direction, increment * DIM);
            }

            // Increment Ray Position
            t += increment;
            position += direction * increment;
            voxel[axis] = next[axis] + boundary[axis] - 1;

            if (color.a > 0.9 || (count > 0 && culled)) { break; }
        }

        return color;
    }
    else
    {
        var hit = Hit(0.0, U32_MAX);

        while (t < t_max) {
            // Get the next voxel boundary
            let next = voxel + boundary;

            // Get minimum distance till next voxel boundaries
            let d = abs((vec3<f32>(next) * DIM_INV - position) * delta);

            // Get axis of smallest distance
            let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

            // Get Increment
            let increment = max(d[axis], 1E-5);

            let count = textureLoad(COUNT, voxel, 0).x;
            let culled = textureLoad(CULLING, voxel, 0).x < 0.5;

            // Accumulate Color
            if (count > 0 && !culled) {
                hit = visit_opaque(count, voxel, origin * DIM, direction, (t + increment) * DIM);
            }

            // Increment Ray Position
            t += increment;
            position += direction * increment;
            voxel[axis] = next[axis] + boundary[axis] - 1;

            if (hit.index != U32_MAX) { break; }
        }

        if (hit.index == U32_MAX) { return vec4<f32>(0.0); }

        let v0 = unpack_vertex(LINE_VERTEX[hit.index + 0]);
        let v1 = unpack_vertex(LINE_VERTEX[hit.index + 1]);

        let position = origin * DIM + direction * hit.distance;

        return shade(v0, v1, RADIUS, position, direction, position * DIM_INV, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);
    }
}

fn visit_transparency(count: u32, voxel: vec3<u32>, origin: vec3<f32>, direction: vec3<f32>, increment: f32) -> vec4<f32> {
    let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;

    let increment_inv = 1.0 / increment;

    var hit_count = 0u;
    var hits = array<u32, LOCAL_SORT_SIZE>();

    for (var i = 0u; i < count; i++) {
        let index = INDEX[offset + i];

        let hit = hittest(index, origin, direction, increment);

        if (hit < 0.0) { continue; }

        let candidate = (u32((hit * increment_inv) * U16_MAX_f32) << 16) | i;

        insert_hit(&hits, hit_count, candidate);

        hit_count++;
    }

    if (hit_count > 0) {
        var color = vec4<f32>(0.0);

        for (var i=0u; i<hit_count; i++) {
            let item = hits[i];
            let index = INDEX[offset + (item & U16_MAX)];

            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            let t_min = f32(item >> 16) * U16_MAX_INV * increment;

            let position = origin + t_min * direction;

            if (should_be_clipped(v0, v1, position)) { continue; }

            let c = shade(v0, v1, RADIUS, position, direction, position * DIM_INV, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);

            color += (1.0 - color.a) * vec4<f32>(c.rgb * c.a, c.a);

            if (color.a > 0.99) { break; }
        }

        return color;
    }

    return vec4<f32>(0.0);
}

fn visit_opaque(count: u32, voxel: vec3<u32>, origin: vec3<f32>, direction: vec3<f32>, increment: f32) -> Hit {
    let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;

    var hit = Hit(increment, U32_MAX);

    for (var i = 0u; i < count; i++) {
        let index = INDEX[offset + i];

        let v0 = LINE_VERTEX[index + 0];
        let v1 = LINE_VERTEX[index + 1];

        let intersection = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);

        if (intersection < hit.distance) {
            hit = Hit(intersection, index);
        }
    }

    return hit;
}

fn hittest(index: u32, origin: vec3<f32>, direction: vec3<f32>, increment: f32) -> f32 {
    let v0 = LINE_VERTEX[index + 0];
    let v1 = LINE_VERTEX[index + 1];

    let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);

    return select(hit, -1.0, hit >= increment);
}

fn anyhit(origin: vec3<f32>, direction: vec3<f32>) -> f32 {
    let max_distance = ENVIRONMENT.settings.shadows;

    if (max_distance == 0.0 || ENVIRONMENT.settings.alpha != 1.0) { return 1.0; }

    let delta = select(1.0 / direction, vec3<f32>(1E10), abs(direction) < vec3<f32>(1E-5));
    let boundary = select(vec3<u32>(0), vec3<u32>(1), direction >= vec3<f32>(0.0));

    var t = 0.0;
    var position = origin;
    var voxel = vec3<u32>(floor(position * DIM));

    while (t < max_distance) {
        // Get the next voxel boundary
        let next = voxel + boundary;

        // Get minimum distance till next voxel boundaries
        let d = abs((vec3<f32>(next) * DIM_INV - position) * delta);

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = max(d[axis], 1E-5);
        let increment_inv = 1.0 / increment;

        // Check Any Hit
        let count = textureLoad(COUNT, voxel, 0).x;
        let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;

        for (var i = 0u; i < count; i++) {
            let index = INDEX[offset + i];

            let hit = hittest(index, position * DIM, direction, increment * DIM);

            if (hit > 0.0) { return smoothstep(0.0, 1.0, t / max_distance); }
        }

        // Increment Ray Position
        t += increment;
        position += direction * increment;
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }

    return 1.0;
}

fn sort(data: ptr<function, array<u32, LOCAL_SORT_SIZE>>, count: u32) {
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

fn binary_search_insert_index(hits: ptr<function, array<u32, LOCAL_SORT_SIZE>>, hit_count: u32, value: u32) -> u32 {
    var lo: u32 = 0u;
    var hi: u32 = hit_count;

    while (lo < hi) {
        let mid: u32 = (lo + hi) / 2u;
        if ((*hits)[mid] < value) {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }

    return lo; // insertion point
}

fn insert_hit(hits: ptr<function, array<u32, LOCAL_SORT_SIZE>>, hit_count: u32, value: u32) {
    let hit_count_clamped = min(hit_count, LOCAL_SORT_SIZE);

    let idx = binary_search_insert_index(hits, hit_count_clamped, value);

    // Shift elements to make room
    for (var i = hit_count_clamped; i > idx; i--) {
        (*hits)[i] = (*hits)[i - 1u];
    }

    // Insert and increment count
    (*hits)[idx] = value;
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
