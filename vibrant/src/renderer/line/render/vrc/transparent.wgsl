@group(2) @binding(1) var<storage, read_write> VERTICES: array<vec3<u32>>;

@group(3) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(3) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

var<private> COLOR: vec4<f32>;

fn visit(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32,
    axis: u32) -> bool {

    if (ENVIRONMENT.settings.shadows > 0.5) {
        return visit_neighborhood(voxel, origin, direction, position, increment, distance);
    } else {
        return visit_no_neighborhood(voxel, origin, direction, position, increment, distance, axis);
    }
}

fn visit_neighborhood(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32) -> bool {

    let increment_inv = 1.0 / increment;

    var hits = array<u32, INSERTION_SORT_SIZE>();
    var hit_count = 0u;

    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let vxl = vec3<u32>(vec3<i32>(voxel) + vec3<i32>(x, y, z));

                let start = textureLoad(START, vxl).x;
                let end = textureLoad(END, vxl).x;

                let count = end - start;

                if (count == 0 || end - start > 512) { continue; }

                for (var i = 0u; i < count; i++) {
                    let index = start + i;

                    let v0 = decode_segment_vertex(vxl, VERTICES[index][0], DIM_INV);
                    let v1 = decode_segment_vertex(vxl, VERTICES[index][1], DIM_INV);

                    // Skip degenerate segments
                    if (all(v0.xyz == v1.xyz)) { continue; }

                    let hit = capsule_intersection(position, direction, v0.xyz, v1.xyz, RADIUS);
                    let hit_position = position + hit * direction;

                    if (hit < 0.0 || hit >= increment || should_be_clipped(v0, v1, hit_position)) { continue; }

                    let depth = u32(saturate(hit * increment_inv) * U16_MAX_f32);

                    let offset = (u32(x + 1) << 4) | (u32(y + 1) << 2) | u32(z + 1);

                    let candidate = (depth << 16) | (i << 6) | offset;

                    insertion_sort_insert(&hits, hit_count, candidate);

                    hit_count++;
                }
            }
        }
    }

    let hit_count_clamped = min(hit_count, INSERTION_SORT_SIZE);

    for (var i=0u; i<hit_count_clamped; i++) {
        let item = hits[i];

        let offset = vec3<i32>(i32((item >> 4) & 3) - 1, i32((item >> 2) & 3) - 1, i32(item & 3) - 1);
        let vxl = vec3<u32>(vec3<i32>(voxel) + offset);

        let start = textureLoad(START, vxl).x;
        let index = start + ((item >> 6) & U8_MAX);

        let v0 = decode_segment_vertex(vxl, VERTICES[index][0], DIM_INV);
        let v1 = decode_segment_vertex(vxl, VERTICES[index][1], DIM_INV);

        let hit = f32(item >> 16u) * U16_MAX_INV * increment;
        let hit_position = position + hit * direction;

        let c = shade(v0, v1, RADIUS, hit_position, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);

        COLOR += (1.0 - COLOR.a) * vec4<f32>(c.rgb * c.a, c.a);

        if (COLOR.a > 0.95) { break; }
    }

    return COLOR.a > 0.95;
}

var<private> LAST_LAST_BITMASK: u32;
var<private> LAST_BITMASK: u32;

fn visit_no_neighborhood(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32,
    axis: u32) -> bool {

    let axis_direction = direction[axis] > 0.0;

    let start = textureLoad(START, voxel).x;
    let end = textureLoad(END, voxel).x;

    let count = end - start;

    if (count == 0 || end - start > 512) { return false; }

    let max_distance = 2.0 * distance;
    let max_distance_inv = 1.0 / max_distance;

    var hits = array<u32, INSERTION_SORT_SIZE>();
    var hit_count = 0u;

    let last_bitmask = LAST_BITMASK | LAST_LAST_BITMASK;
    var bitmask = 0u;

    for (var i = 0u; i < count; i++) {
        let index = start + i;

        let v0 = decode_segment_vertex(voxel, VERTICES[index][0], DIM_INV);
        let v1 = decode_segment_vertex(voxel, VERTICES[index][1], DIM_INV);

        let line_bit = 1u << (VERTICES[index][2] & 31);

        // Skip degenerate segments & lines we've already processed
        if (all(v0.xyz == v1.xyz) || (last_bitmask & line_bit) != 0u) { continue; }

        let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);
        let hit_position = origin + hit * direction;

        if (hit >= max_distance) { continue; }

        bitmask |= line_bit;

        let candidate = (u32(saturate(hit * max_distance_inv) * U16_MAX_f32) << 16) | i;

        insertion_sort_insert(&hits, hit_count, candidate);

        hit_count++;
    }

    LAST_LAST_BITMASK = LAST_BITMASK;
    LAST_BITMASK = bitmask;

    let hit_count_clamped = min(hit_count, INSERTION_SORT_SIZE);

    for (var i=0u; i<hit_count_clamped; i++) {
        let item = hits[i];

        let index = start + (item & U8_MAX);

        var v0 = decode_segment_vertex(voxel, VERTICES[index][0], DIM_INV);
        var v1 = decode_segment_vertex(voxel, VERTICES[index][1], DIM_INV);

        let tangent = normalize(v1.xyz - v0.xyz);

        if (!all(v0.clip == vec3<f32>())) { v0.xyz -= tangent; }
        if (!all(v1.clip == vec3<f32>())) { v1.xyz += tangent; }

        let hit = min(
            capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS),
            f32(item >> 16u) * U16_MAX_INV * max_distance);
        let hit_position = origin + hit * direction;

        let c = shade(v0, v1, RADIUS, hit_position, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);

        COLOR += (1.0 - COLOR.a) * vec4<f32>(c.rgb * c.a, c.a);

        if (COLOR.a > 0.95) { break; }
    }

    return COLOR.a > 0.95;
}

fn result(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    return COLOR;
}
