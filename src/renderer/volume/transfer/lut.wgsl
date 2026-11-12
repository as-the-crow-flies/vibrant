use color;
use material;

// Layer 0: (absorption, ior), layer 1: (scattering, density). Bin i is fraction i / 255.
@group(0) @binding(0) var LUT: texture_storage_2d_array<rgba16float, write>;

@group(1) @binding(3) var<uniform> MATERIAL: Material;
@group(1) @binding(4) var COLORMAP: texture_3d<f32>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let bins = textureDimensions(LUT).x;
    if (id.x >= bins) { return; }

    let material = classify(f32(id.x) / f32(bins - 1u));

    textureStore(LUT, vec2<u32>(id.x, 0u), 0, vec4<f32>(material.absorption, material.ior));
    let density = brightness(material.absorption + material.scattering);
    textureStore(LUT, vec2<u32>(id.x, 0u), 1, vec4<f32>(material.scattering, density));
}
