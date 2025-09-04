@group(0) @binding(0) var<storage, read_write> VERTICES: array<u32>;

@group(1) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(1) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

@group(2) @binding(0) var DENSITY: texture_storage_3d<r32float, read_write>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.14159265358979323846264338327950288;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let radius = ENVIRONMENT.settings.radius;
    let alpha = ENVIRONMENT.settings.alpha;

    let density_multiplier = PI * radius * radius * alpha;

    let start = textureLoad(START, voxel).x;
    let end = textureLoad(END, voxel).x;

    var density = 0.0;

    for (var index = start; index < end; index++) {
        let segment = decode_segment(voxel, VERTICES[index]);
        density += length(segment.v1 - segment.v0);
    }

    textureStore(DENSITY, voxel, vec4<f32>(saturate(density * density_multiplier)));
}
