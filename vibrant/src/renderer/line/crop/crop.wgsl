@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
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

    if (index >= n_lines) { return; }

    let start = LINE_OFFSET_RAW[index];
    let end = LINE_OFFSET_RAW[index + 1];

    let visible = LINE_SETTINGS[LINE_MATERIAL[LINE_INDEX_RAW[start]]].visible == TRUE;

    if (!visible) { return; }

    let length = end - start;

    let offset_start = u32(ENVIRONMENT.settings.crop_start * f32(length));
    let offset_end = u32(ENVIRONMENT.settings.crop_end * f32(length));

    let crop_length = min(offset_end - offset_start, length);

    let crop_min = vec3<f32>(
        ENVIRONMENT.settings.crop_x_start,
        ENVIRONMENT.settings.crop_y_start,
        ENVIRONMENT.settings.crop_z_start
    );

    let crop_max = vec3<f32>(
        ENVIRONMENT.settings.crop_x_end,
        ENVIRONMENT.settings.crop_y_end,
        ENVIRONMENT.settings.crop_z_end
    );

    var total_length = 0u;
    for (var i=0u; i<crop_length; i++) {
        let index = LINE_INDEX_RAW[start + offset_start + i];
        let v0 = LINE_VERTEX[index].xyz;
        let v1 = LINE_VERTEX[index + 1].xyz;

        if(all(v0 >= crop_min) && all(v0 <= crop_max) &&
           all(v1 >= crop_min) && all(v1 <= crop_max)) {
           total_length++;
        }
    }

    let offset_line = atomicAdd(&LINE_LENGTH, total_length);

    var offset_index = 0u;

    for (var i=0u; i<crop_length; i++) {
        let index = LINE_INDEX_RAW[start + offset_start + i];
        let v0 = LINE_VERTEX[index].xyz;
        let v1 = LINE_VERTEX[index + 1].xyz;

        if(all(v0 >= crop_min) && all(v0 <= crop_max) &&
           all(v1 >= crop_min) && all(v1 <= crop_max)) {
            LINE_INDEX[offset_line + offset_index] = LINE_INDEX_RAW[start + offset_start + i];
            offset_index++;
        }
    }
}
