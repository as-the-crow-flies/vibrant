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

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;
