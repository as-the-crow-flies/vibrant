@group(0) @binding(0) var<storage> LINE_INDEX_RAW: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX_RAW: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> TRANSFORM: mat4x4<f32>;

@group(1) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(1) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read_write> LINE_LENGTH: u32;

@group(1) @binding(7) var<storage> LINE_LENGTH: array<u32>;
@group(1) @binding(8) var<storage> LINE_OFFSET: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(global_invocation_index) global: vec3<u32>) {
    let line_index = global.x;

    if (line_index >= arrayLength(LINE_LENGTH)) { return; }

    let sphere_position = vec3<f32>(0.0);
    let sphere_radius = 1.0;

    let line_start = LINE_OFFSET;
    let line_end = line_start + LINE_LENGTH;

    for (var i = line_start; i < line_end; i++) {
    }
}
