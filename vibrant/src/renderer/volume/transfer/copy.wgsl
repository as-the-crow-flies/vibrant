@group(0) @binding(0) var ABSORPTION_SRC: texture_storage_3d<r32uint, read>;
@group(0) @binding(1) var SCATTERING_SRC: texture_storage_3d<r32uint, read>;
@group(0) @binding(2) var PROPERTIES_SRC: texture_storage_3d<r32uint, read>;

@group(0) @binding(3) var ABSORPTION_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(4) var SCATTERING_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(5) var EXTINCTION_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(6) var PROPERTIES_DST: texture_storage_3d<rgba8unorm, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(ABSORPTION_SRC))) { return; }

    let absorption = unpack4x8unorm(textureLoad(ABSORPTION_SRC, voxel).x);
    let scattering = unpack4x8unorm(textureLoad(SCATTERING_SRC, voxel).x);
    let properties = unpack4x8unorm(textureLoad(PROPERTIES_SRC, voxel).x);

    textureStore(ABSORPTION_DST, voxel, absorption);
    textureStore(SCATTERING_DST, voxel, scattering);
    textureStore(EXTINCTION_DST, voxel, pack_rgb(unpack_rgb(absorption) + unpack_rgb(scattering)));
    textureStore(PROPERTIES_DST, voxel, properties);
}
