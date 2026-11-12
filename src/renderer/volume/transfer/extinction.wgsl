use classify;

// The cascades' extinction: the tracer's classification at each voxel center.
@group(0) @binding(0) var EXTINCTION: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let size = textureDimensions(EXTINCTION);
    if (any(voxel >= size)) { return; }

    let material = classify((vec3<f32>(voxel) + 0.5) / vec3<f32>(size));
    textureStore(EXTINCTION, voxel, vec4<f32>(material.absorption + material.scattering, 0.0));
}
