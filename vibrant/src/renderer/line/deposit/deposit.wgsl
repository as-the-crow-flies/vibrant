// One invocation per PhysicalVolume voxel. Samples the resolution-independent
// line occupancy density pyramid and writes a neutral (uncolored) extinction
// scalar into `line_extinction` mip 0. A mipmap pass then builds its chain so
// `cascade.wgsl`'s coarse `cull()` LOD lookups see the lines. The volume tracer
// never touches this texture.

@group(0) @binding(0) var LINE_EXTINCTION: texture_storage_3d<r32float, read_write>;
@group(0) @binding(1) var LINE_SAMPLER: sampler; // unused (layout_write entry)

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var DENSITY_SAMPLER: sampler;

const LINE_DENSITY_SCALE: f32 = 4.0;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = textureDimensions(LINE_EXTINCTION);
    if (any(voxel >= dim)) { return; }

    let uv = (vec3<f32>(voxel) + 0.5) / vec3<f32>(dim);

    // Box-average the occupancy pyramid over this voxel's footprint when we're
    // coarser than it. Point-sampling a sparse density field at a lower
    // resolution loses optical depth proportional to the resolution gap;
    // averaging keeps the deposited density independent of it.
    let ratio = vec3<f32>(textureDimensions(DENSITY)) / vec3<f32>(dim);
    let mip = max(log2(max(ratio.x, max(ratio.y, ratio.z))), 0.0);
    let density = max(textureSampleLevel(DENSITY, DENSITY_SAMPLER, uv, mip).x, 0.0);

    // Written every voxel (empty ones get 0) so the mip chain is well-defined.
    textureStore(LINE_EXTINCTION, voxel, vec4<f32>(density * LINE_DENSITY_SCALE, 0.0, 0.0, 0.0));
}
