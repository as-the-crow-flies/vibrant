@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;

@group(0) @binding(5) var<storage> LINE_LENGTH: array<u32>;
@group(0) @binding(6) var<storage> LINE_OFFSET: array<u32>;

@group(1) @binding(0) var<storage, read_write> KEY: array<u32>;
@group(1) @binding(1) var<storage, read_write> VALUE: array<u32>;
@group(1) @binding(2) var<storage, read_write> COUNT: atomic<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

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

    var last_axis = 0u;
    var last_position = vec3<f32>(-1.0);

    for (var i = line_start; i < line_end - 1; i++) {
        let index = LINE_INDEX[i];

        let v0 = (LINE_VERTEX[index + 0].xyz + 0.5) * scale;
        let v1 = (LINE_VERTEX[index + 1].xyz + 0.5) * scale;

        quantize2(v0, v1, &last_position, &last_axis);
    }
}

fn quantize2(v0: vec3<f32>, v1: vec3<f32>, last_position: ptr<function, vec3<f32>>, last_axis: ptr<function, u32>) {
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

        if (t < distance) {
            if (last_position.x >= 0.0) {
                let index = atomicAdd(&COUNT, 1u);
                KEY[index] = encode_segment(voxel, *last_position, *last_axis, position, axis);
                VALUE[index] = morton_encode(voxel);
            }

            *last_position = position;
            *last_axis = axis;
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
