@group(0) @binding(0) var<storage> LINE_INDEX_RAW: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX_RAW: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> TRANSFORM: mat4x4<f32>;

@group(1) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(1) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read_write> LINE_TOTAL_COUNT: atomic<u32>;

@group(1) @binding(5) var<storage> LINE_LENGTH: array<u32>;
@group(1) @binding(6) var<storage> LINE_OFFSET: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(global_invocation_id) global: vec3<u32>) {
    let n_lines = arrayLength(&LINE_LENGTH);

    let radius = ENVIRONMENT.settings.radius / f32(ENVIRONMENT.volume);

    let line_index = global.x;

    if (line_index >= n_lines) { return; }

    let sphere_position = vec3<f32>(0.0);
    let sphere_radius = 0.1;

    let line_length = LINE_LENGTH[line_index];
    let line_start = LINE_OFFSET[line_index];
    let line_end = line_start + line_length;

    var included = line_index < u32(ENVIRONMENT.settings.shadows * f32(n_lines));

    // for (var i = line_start; i < line_end; i++) {
    //     let vertex = LINE_VERTEX[LINE_INDEX_RAW[i]].xyz;

    //     if (length(vertex - sphere_position) <= sphere_radius + radius) {
    //         included = true;
    //         break;
    //     }
    // }

    if (included) {
        let offset = atomicAdd(&LINE_TOTAL_COUNT, line_length);

        for (var i = 0u; i < line_length; i++) {
            LINE_INDEX[offset + i] = LINE_INDEX_RAW[line_start + i];
        }
    }
}
