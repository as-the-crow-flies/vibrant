@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> LINE_LENGTH: atomic<u32>;

@group(0) @binding(4) var<storage> LINE_MATERIAL: array<u32>;
@group(0) @binding(5) var<storage> LINE_SETTINGS: array<LineSettings>;

@group(0) @binding(6) var<storage> LINE_INDEX_RAW: array<u32>;
@group(0) @binding(7) var<storage> LINE_VERTEX_RAW: array<vec4<f32>>;
@group(0) @binding(8) var<storage> LINE_OFFSET_RAW: array<u32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

var<workgroup> OFFSET: u32;

//DISPATCH-INSERT-MARKER//

@compute
@workgroup_size(32)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let n_lines = arrayLength(&LINE_OFFSET_RAW);
    let index = global_invocation_id.x;

    if (index >= n_lines - 1) { return; }

    let start = LINE_OFFSET_RAW[index];
    let end = LINE_OFFSET_RAW[index + 1];

    let settings = LINE_SETTINGS[LINE_MATERIAL[LINE_INDEX_RAW[start]]];

    let visible = settings.visible == TRUE;

    if (!visible) { return; }

    let length = end - start;

    let start_index = LINE_INDEX_RAW[start];
    let t = ENVIRONMENT.time;

    let crop_start = max(ENVIRONMENT.settings.crop_start, settings.crop_start);
    let crop_end = min(ENVIRONMENT.settings.crop_end, settings.crop_end);

    let offset_start = u32(crop_start * f32(length));
    let offset_end = u32(crop_end * f32(length));

    let crop_length = min(offset_end - offset_start, length);

    if (crop_length == 0) { return; }

    let selection_scale = ENVIRONMENT.settings.selection_scale;

    if (selection_scale <= 0.0) {
        let offset_line = atomicAdd(&LINE_LENGTH, crop_length);
        for (var i=0u; i<crop_length; i++) {
            LINE_INDEX[offset_line + i] = LINE_INDEX_RAW[start + offset_start + i];
        }
        return;
    }

    //DISPATCH-CALL-MARKER//
    if ENVIRONMENT.settings.selection_match_all {
        let a = in_square_volume(
            ENVIRONMENT.settings.selection_scale,
            ENVIRONMENT.settings.selection_offset_x,
            ENVIRONMENT.settings.selection_offset_y,
            ENVIRONMENT.settings.selection_offset_z,
            crop_length, start, offset_start
        );
        let b = in_square_volume(
            0.5, 0.125, 0.125, 0.125,
            crop_length, start, offset_start
        );

        if (!a || !b) { return; }
    }

    let extend = ENVIRONMENT.settings.selection_extend_lines == TRUE;

    if (extend) {
        let offset_line = atomicAdd(&LINE_LENGTH, crop_length);
        for (var i = 0u; i < crop_length; i++) {
            LINE_INDEX[offset_line + i] = LINE_INDEX_RAW[start + offset_start + i];
        }
    } else {
        var total_length = 0u;
        for (var i = 0u; i < crop_length; i++) {
            let idx = LINE_INDEX_RAW[start + offset_start + i];
            if (
                //DISPATCH-PARTIAL-CALL-MARKER//
            ) {
                total_length++;
            }
        }
        let offset_line = atomicAdd(&LINE_LENGTH, total_length);
        var offset_index = 0u;
        for (var i = 0u; i < crop_length; i++) {
            let idx = LINE_INDEX_RAW[start + offset_start + i];
            if (
                //DISPATCH-PARTIAL-CALL-MARKER//
            ) {
                LINE_INDEX[offset_line + offset_index] = LINE_INDEX_RAW[start + offset_start + i];
                offset_index++;
            }
        }
    }
}