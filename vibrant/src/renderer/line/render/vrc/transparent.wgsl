@group(2) @binding(1) var<storage, read_write> VERTICES: array<vec2<u32>>;

@group(3) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(3) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

var<private> COLOR: vec4<f32>;

fn visit(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32) -> bool {

    let start = textureLoad(START, voxel).x;
    let end = textureLoad(END, voxel).x;

    let count = end - start;

    if (count == 0 || end - start > 512) { return false; }

    let max_distance = 2.0 * distance;
    let max_distance_inv = 1.0 / max_distance;

    var hits = array<u32, INSERTION_SORT_SIZE>();
    var hit_count = 0u;

    for (var i = 0u; i < count; i++) {
        let index = start + i;

        let v0 = decode_segment_vertex(voxel, VERTICES[index][0], DIM_INV);
        let v1 = decode_segment_vertex(voxel, VERTICES[index][1], DIM_INV);

        // Skip degenerate segments
        if (all(v0.xyz == v1.xyz)) { continue; }

        let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);
        let hit_position = origin + hit * direction;

        if (hit >= max_distance || should_be_clipped(v0, v1, hit_position)) { continue; }

        let candidate = (u32(saturate(hit * max_distance_inv) * U16_MAX_f32) << 16) | i;

        insertion_sort_insert(&hits, hit_count, candidate);

        hit_count++;
    }

    let hit_count_clamped = min(hit_count, INSERTION_SORT_SIZE);

    for (var i=0u; i<hit_count_clamped; i++) {
        let item = hits[i];

        let index = start + (item & U8_MAX);

        let v0 = decode_segment_vertex(voxel, VERTICES[index][0], DIM_INV);
        let v1 = decode_segment_vertex(voxel, VERTICES[index][1], DIM_INV);

        let hit = f32(item >> 16u) * U16_MAX_INV * max_distance;
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
