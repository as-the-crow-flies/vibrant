use math;
use uniforms;
use vertex;

@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> LINE_LENGTH: atomic<u32>;
@group(0) @binding(3) var<storage> LINE_MATERIAL: array<u32>;
@group(0) @binding(4) var<storage> LINE_SETTINGS: array<LineSettings>;
@group(0) @binding(5) var<storage> LINE_VERTEX_RAW: array<vec4<f32>>;
// Line of each vertex, and the first vertex of each line (+ end sentinel).
@group(0) @binding(6) var<storage> LINE_ID: array<u32>;
@group(0) @binding(7) var<storage> LINE_START: array<u32>;
@group(0) @binding(8) var<uniform> TRANSFORM: mat4x4<f32>;

@group(1) @binding(0) var<uniform> TRACTOGRAPHY: Tractography;

@group(2) @binding(0) var<uniform> CROP: CropSettings;

const WORKGROUP_SIZE: u32 = 256;

var<workgroup> COUNT: atomic<u32>;
var<workgroup> BASE: u32;

// One thread per vertex: transform it, write its clip plane, and append the
// segment starting at it to LINE_INDEX if that segment survives cropping.
@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
    @builtin(local_invocation_index) local: u32,
) {
    let n_vertices = arrayLength(&LINE_VERTEX_RAW);
    let crop_normal = normal_from_spherical(CROP.spherical.x, CROP.spherical.y);

    for (var base = workgroup_id.x * WORKGROUP_SIZE; base < n_vertices; base += num_workgroups.x * WORKGROUP_SIZE) {
        let i = base + local;

        var keep = false;
        if (i < n_vertices) { keep = prepare(i, n_vertices, crop_normal); }

        if (local == 0) { atomicStore(&COUNT, 0u); }
        workgroupBarrier();

        var slot = 0u;
        if (keep) { slot = atomicAdd(&COUNT, 1u); }
        workgroupBarrier();

        if (local == 0) { BASE = atomicAdd(&LINE_LENGTH, atomicLoad(&COUNT)); }
        let offset = workgroupUniformLoad(&BASE);

        if (keep) { LINE_INDEX[offset + slot] = i; }
    }
}

fn prepare(i: u32, n_vertices: u32, crop_normal: vec3<f32>) -> bool {
    let line = LINE_ID[i];
    let start = LINE_START[line];
    let n_segments = LINE_START[line + 1] - start - 1;
    let k = i - start;

    let settings = LINE_SETTINGS[LINE_MATERIAL[i]];

    let raw = LINE_VERTEX_RAW[i];
    let v = transform(raw);
    let vm = transform(LINE_VERTEX_RAW[max(i, 1u) - 1]);
    let vp = transform(LINE_VERTEX_RAW[min(i + 1, n_vertices - 1)]);

    // Segments [first, last) of this line survive the length crop.
    let crop_start = max(TRACTOGRAPHY.crop_start, settings.crop_start);
    let crop_end = min(TRACTOGRAPHY.crop_end, settings.crop_end);
    let first = u32(crop_start * f32(n_segments));
    let last = select(0u, min(u32(crop_end * f32(n_segments)), n_segments), settings.visible == TRUE);

    let inside = is_inside(v, settings, crop_normal);
    let keep_prev = k > first && k - 1 < last && inside && is_inside(vm, settings, crop_normal);
    let keep_next = k >= first && k < last && inside && is_inside(vp, settings, crop_normal);

    let clip = select(vec3<f32>(0), normalize(vp - vm), keep_prev && keep_next);

    LINE_VERTEX[i] = vec4<f32>(v, pack_clip_alpha(vec4<f32>(clip, raw.a)));

    return keep_next;
}

fn transform(raw: vec4<f32>) -> vec3<f32> {
    return (TRANSFORM * vec4<f32>(raw.xyz, 1.0)).xyz - 0.5;
}

fn is_inside(v: vec3<f32>, settings: LineSettings, crop_normal: vec3<f32>) -> bool {
    if (settings.crop == FALSE) { return true; }

    let within_spherical = dot(crop_normal, v) + CROP.spherical.z - 0.5 < 0.0;
    let within_orthogonal = all(v >= CROP.min.xyz) && all(v <= CROP.max.xyz);

    return within_spherical && within_orthogonal;
}

fn normal_from_spherical(phi: f32, theta: f32) -> vec3<f32> {
    let sin_theta = sin(theta);
    return vec3<f32>(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos(theta)
    );
}
