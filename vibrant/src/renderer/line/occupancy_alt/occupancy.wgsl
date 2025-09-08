@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;

@group(1) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;

@group(2) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(2) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

@group(3) @binding(0) var DENSITY: texture_storage_3d<r32float, read_write>;

@group(4) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.14159265358979323846264338327950288;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));

    let scale = f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius;
    let alpha = ENVIRONMENT.settings.alpha;

    let density_multiplier = PI * radius * radius * alpha;

    let start = textureLoad(START, voxel).x;
    let end = textureLoad(END, voxel).x;

    var density = 0.0;

    for (var index = start; index < end; index++) {
        let line_index = LINE_INDEX[index];

        let v0 = unpack_vertex_scale(LINE_VERTEX[line_index + 0], scale);
        let v1 = unpack_vertex_scale(LINE_VERTEX[line_index + 1], scale);

        let smoothing = ENVIRONMENT.settings.smoothing;

        let radius_clamp = max(smoothing, radius);
        let radius_ratio = pow(radius / radius_clamp, 2.0);

        let p = vec3<f32>(voxel) + 0.5;

        let delta = v1.xyz - v0.xyz;
        let pv0 = p.xyz - v0.xyz;
        let pv1 = p.xyz - v1.xyz;

        let height = saturate(dot(pv0, delta) / dot(delta, delta));

        let sdf = max(length(pv0 - delta * height) - radius_clamp, max(-dot(pv0, v0.clip), dot(pv1, v1.clip)));

        density += radius_ratio * mix(v0.alpha, v1.alpha, height) * saturate(0.5 - sdf);
    }

    textureStore(DENSITY, voxel, vec4<f32>(saturate(density * density_multiplier)));
}
