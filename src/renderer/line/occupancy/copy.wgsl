use block;
use math;
use uniforms;

@group(0) @binding(0) var<storage, read_write> DENSITY_COUNT_BUFFER: array<u32>;
@group(1) @binding(0) var DENSITY: texture_storage_3d<r32float, read_write>;
@group(2) @binding(0) var COUNT: texture_storage_3d<r32uint, read_write>;
@group(3) @binding(0) var<uniform> TRACTOGRAPHY: Tractography;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let index = block_index(voxel, textureDimensions(DENSITY));

    let density_count_encoded = DENSITY_COUNT_BUFFER[index];
    let count = density_count_encoded & U16_MAX;

    // Summed tube volume fraction, unclamped: overlapping tubes still add
    // extinction. Kept nonzero wherever a segment was visited, for traversal.
    let fraction = U12_MAX_INV * f32(density_count_encoded >> U16_SHIFT);
    let density = select(0.0, max(fraction, 1e-6), count > 0u);

    textureStore(DENSITY, voxel, vec4<f32>(density));
    textureStore(COUNT, voxel, vec4<u32>(count));
}
