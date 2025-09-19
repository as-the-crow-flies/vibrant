@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;

@group(0) @binding(5) var<storage> LINE_LENGTH: array<u32>;
@group(0) @binding(6) var<storage> LINE_OFFSET: array<u32>;

@group(1) @binding(0) var<storage, read_write> KEY: array<u32>;
@group(1) @binding(1) var<storage, read_write> VALUE: array<vec3<u32>>;
@group(1) @binding(2) var<storage, read_write> COUNT: atomic<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Intersection {
    position: vec3<f32>,
    voxel: vec3<u32>,
    axis: u32,
}

@compute
@workgroup_size(1024)
fn main(@builtin(global_invocation_id) global: vec3<u32>) {
    let n_lines = arrayLength(&LINE_LENGTH);
    let line_index = global.x;

    if (line_index >= n_lines) { return; }

    let scale = f32(ENVIRONMENT.volume);

    let line_length = LINE_LENGTH[line_index];
    let line_start = LINE_OFFSET[line_index];
    let line_end = line_start + line_length;

    var intersections = array<Intersection, 3>();

    for (var i = line_start; i < line_end; i++) {
        let index = LINE_INDEX[i];

        let v0 = (LINE_VERTEX[index + 0].xyz + 0.5) * scale;
        let v1 = (LINE_VERTEX[index + 1].xyz + 0.5) * scale;

        quantize(line_index, v0, v1, &intersections);
    }

    // Quantize final segment
    let v1 = intersections[1];
    let v2 = intersections[2];

    let clip_1 = normalize(v2.position - intersections[0].position);
    let clip_2 = vec3<f32>(0.0);

    let index = atomicAdd(&COUNT, 1u);

    KEY[index] = morton_encode(v2.voxel);
    VALUE[index] = vec3<u32>(
        encode_segment_vertex(v2.voxel, v1.position, v1.axis, clip_1),
        encode_segment_vertex(v2.voxel, v2.position, v2.axis, clip_2),
        line_index
    );
}

fn quantize(line_index: u32, v0: vec3<f32>, v1: vec3<f32>, intersections: ptr<function, array<Intersection, 3>>) {
    let scale = 1.0 / f32(ENVIRONMENT.volume);

    let delta = v1 - v0;

    let distance = length(delta);
    let direction = delta / distance;

    let step = select(1.0 / direction, vec3<f32>(1E10), abs(direction) < vec3<f32>(1E-5));
    let boundary = select(vec3<u32>(0), vec3<u32>(1), direction >= vec3<f32>(0.0));

    var t = 0.0;
    var position = v0;
    var voxel = vec3<u32>(v0);

    while (t < distance) {
        // Get the next voxel boundary
        let next = voxel + boundary;

        // Get minimum distance till next voxel boundaries
        let d = one_if_zero(abs((vec3<f32>(next) - position) * step));

        // Get axis of smallest distance
        let axis = select(select(2u, 1u, d.y < d.z), 0u, d.x < min(d.y, d.z));

        // Get Increment
        let increment = d[axis];

        // Increment Line Position
        t += increment;
        position += direction * increment;

        if (t > 0.0 && t < distance) {
            if (intersections[1].position.x != 0.0) {
                let v1 = intersections[1];
                let v2 = intersections[2];

                let clip_1 = select(
                    normalize(v2.position - intersections[0].position),
                    vec3<f32>(0.0),
                    intersections[0].position.x == 0.0);
                let clip_2 = normalize(position - v1.position);

                let index = atomicAdd(&COUNT, 1u);
                KEY[index] = morton_encode(v2.voxel);
                VALUE[index] = vec3<u32>(
                    encode_segment_vertex(v2.voxel, v1.position, v1.axis, clip_1),
                    encode_segment_vertex(v2.voxel, v2.position, v2.axis, clip_2),
                    line_index
                );
            }

            intersections[0] = intersections[1];
            intersections[1] = intersections[2];
            intersections[2] = Intersection(position, voxel, axis);
        }

        // Increment Voxel
        voxel[axis] = next[axis] + boundary[axis] - 1;
    }
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v == vec3<f32>(0.0));
}

fn minimum(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}
