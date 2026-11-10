use uniforms;

@group(0) @binding(0) var LINE_EXTINCTION: texture_storage_3d<r32float, read_write>;
@group(0) @binding(1) var LINE_SAMPLER: sampler; // unused (layout_write entry)

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var DENSITY_SAMPLER: sampler;

@group(2) @binding(0) var<uniform> TRACTOGRAPHY: Tractography;

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
    let fraction = textureSampleLevel(DENSITY, DENSITY_SAMPLER, uv, mip).x;

    // Blocked share of the texel as optical depth, so full coverage is opaque.
    // Clamped at the occupancy saturation, 1 - e^-8.
    let depth = -log(1.0 - min(fraction, 0.99966454));

    // σ = depth / (2 min(r, h)): thin-rod medium for tubes below the
    // voxelization's resolution h. `line_extinction` is 1 / (2 r). Translucent
    // fibres pass their transmitted share t on, treated as straight-through,
    // and partially opaque ones block only their `alpha` share.
    let resolution = max(1.0, TRACTOGRAPHY.smoothing);
    let transmitted = (1.0 - TRACTOGRAPHY.line_translucency) * TRACTOGRAPHY.alpha;
    let sigma = TRACTOGRAPHY.line_extinction * max(1.0, TRACTOGRAPHY.radius / resolution) * transmitted * depth;

    // Written every voxel (empty ones get 0) so the mip chain is well-defined.
    textureStore(LINE_EXTINCTION, voxel, vec4<f32>(sigma, 0.0, 0.0, 0.0));
}
