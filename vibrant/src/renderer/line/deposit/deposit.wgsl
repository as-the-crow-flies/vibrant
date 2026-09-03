// One invocation per PhysicalVolume voxel. Samples the resolution-independent
// line occupancy density pyramid and writes a neutral (uncolored) extinction
// scalar into `line_extinction` mip 0. A mipmap pass then builds its chain so
// `cascade.wgsl`'s coarse `cull()` LOD lookups see the lines. The volume tracer
// never touches this texture.

@group(0) @binding(0) var LINE_EXTINCTION: texture_storage_3d<r32float, read_write>;
@group(0) @binding(1) var LINE_SAMPLER: sampler; // unused (layout_write entry)

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var DENSITY_SAMPLER: sampler;

const LINE_DENSITY_SCALE: f32 = 1.0;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = textureDimensions(LINE_EXTINCTION);
    if (any(voxel >= dim)) { return; }

    let uv = (vec3<f32>(voxel) + 0.5) / vec3<f32>(dim);
    let density = max(textureSampleLevel(DENSITY, DENSITY_SAMPLER, uv, 0.0).x, 0.0);

    // Written every voxel (empty ones get 0) so the mip chain is well-defined.
    textureStore(LINE_EXTINCTION, voxel, vec4<f32>(density * LINE_DENSITY_SCALE, 0.0, 0.0, 0.0));
}
