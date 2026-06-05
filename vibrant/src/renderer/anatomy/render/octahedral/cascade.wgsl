@group(0) @binding(0) var RADIANCE_OUT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(1) var TRANSMISSION_OUT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(2) var RADIANCE_IN: texture_3d<f32>;
@group(0) @binding(3) var TRANSMISSION_IN: texture_3d<f32>;
@group(0) @binding(4) var<uniform> CASCADE: u32;

@group(1) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(1) @binding(1) var SCATTERING: texture_3d<f32>;
@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var GRADIENT: texture_3d<f32>;
@group(1) @binding(4) var SAMPLER: sampler;
@group(1) @binding(5) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(6) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {

}
