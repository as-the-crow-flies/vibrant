@group(0) @binding(0) var<storage, read_write> DENSITY_COUNT_BUFFER: array<u32>;
@group(0) @binding(1) var<storage, read_write> COLOR_BUFFER: array<u32>;

@group(1) @binding(0) var DENSITY: texture_storage_3d<r8unorm, read_write>;
@group(2) @binding(0) var COUNT: texture_storage_3d<r16uint, read_write>;
@group(3) @binding(0) var COLOR: texture_storage_3d<rgba8unorm, read_write>;
@group(4) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let index = block_index(voxel, textureDimensions(DENSITY));

    let density_count = DENSITY_COUNT_BUFFER[index];

    let density = precision_encode(saturate(U16_MAX_INV * f32(density_count >> U14_SHIFT)));
    let count = density_count & U14_MAX;

    textureStore(DENSITY, voxel, vec4<f32>(density));
    textureStore(COUNT, voxel, vec4<u32>(count));

    if (ENVIRONMENT.settings.alpha == 1.0) { return; }

    let color_u32 = vec3<u32>(
        COLOR_BUFFER[index * 3 + 0],
        COLOR_BUFFER[index * 3 + 1],
        COLOR_BUFFER[index * 3 + 2]
    );

    let color = vec3<f32>(color_u32) * U24_MAX_INV;

    textureStore(COLOR, voxel, vec4<f32>(color / max(1.0, length(color)), density));
}
