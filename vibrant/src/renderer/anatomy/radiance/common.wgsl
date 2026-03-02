struct RadianceInterval {
    radiance: vec3<f32>,
    transmission: vec3<f32>
}

const STEP_SIZE: f32 = 1.0;

@group(0) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING: texture_3d<f32>;
@group(0) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(0) @binding(3) var GRADIENT: texture_3d<f32>;
@group(0) @binding(4) var SAMPLER: sampler;
@group(0) @binding(5) var<uniform> TRANSFORM: mat4x4<f32>;
@group(0) @binding(6) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

const FRAME_POS_X: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 1.0, 0.0, 0.0), vec3<f32>( 0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
const FRAME_POS_Y: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0, 1.0, 0.0), vec3<f32>( 1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
const FRAME_POS_Z: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0, 0.0, 1.0), vec3<f32>( 1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0));
const FRAME_NEG_X: mat3x3<f32> = mat3x3<f32>(vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>( 0.0,-1.0, 0.0), vec3<f32>(0.0, 0.0,-1.0));
const FRAME_NEG_Y: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0,-1.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0,-1.0));
const FRAME_NEG_Z: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0, 0.0,-1.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0,-1.0, 0.0));
