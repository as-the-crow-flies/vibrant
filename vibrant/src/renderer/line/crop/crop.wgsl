@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(2) var<storage, read_write> LINE_LENGTH: atomic<u32>;

@group(0) @binding(4) var<storage> LINE_MATERIAL: array<u32>;
@group(0) @binding(5) var<storage> LINE_SETTINGS: array<LineSettings>;

@group(0) @binding(6) var<storage> LINE_INDEX_RAW: array<u32>;
@group(0) @binding(7) var<storage> LINE_OFFSET_RAW: array<u32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

var<workgroup> OFFSET: u32;

@compute
@workgroup_size(32)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let n_lines = arrayLength(&LINE_OFFSET_RAW);
    let index = global_invocation_id.x;

    if (index >= n_lines - 1) { return; }

    let start = LINE_OFFSET_RAW[index];
    let end = LINE_OFFSET_RAW[index + 1];

    let visible = LINE_SETTINGS[LINE_MATERIAL[LINE_INDEX_RAW[start]]].visible == TRUE;

    if (!visible) { return; }

    let length = end - start;

    let offset_start = u32(ENVIRONMENT.settings.crop_start * f32(length));
    let offset_end = u32(ENVIRONMENT.settings.crop_end * f32(length));

    let crop_length = min(offset_end - offset_start, length);

    let offset = atomicAdd(&LINE_LENGTH, crop_length);

    for (var i=0u; i<crop_length; i++) {
        LINE_INDEX[offset + i] = LINE_INDEX_RAW[start + offset_start + i];
    }
}
