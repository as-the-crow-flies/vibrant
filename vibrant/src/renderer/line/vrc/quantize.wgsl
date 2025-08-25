@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(0) @binding(7) var<storage> LINE_LENGTH: array<u32>;
@group(0) @binding(8) var<storage> LINE_OFFSET: array<u32>;

@group(1) @binding(0) var<storage, read_write> KEY: array<u32>;
@group(1) @binding(1) var<storage, read_write> VALUE: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(1024)
fn main(@builtin(global_invocation_id) global: u32) {
    let line_id = global.x;

    if (line_id >= arrayLength(LINE_LENGTH)) { return; }



}
