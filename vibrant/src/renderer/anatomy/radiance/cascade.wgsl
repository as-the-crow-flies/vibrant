@group(0) @binding(0) var ABSORPTION_TRANSMISSION: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING_ROUGHNESS: texture_3d<f32>;
@group(0) @binding(2) var SAMPLER: sampler;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(2) @binding(0) var CASCADE_IN: texture_3d<f32>;
@group(3) @binding(0) var CASCADE_OUT: texture_storage_3d<rgba8unorm, write>;



@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
}
