struct Direction {
    normal: vec4<f32>,
    tangent: vec4<f32>,
    bitangent: vec4<f32>
}

struct RadianceInterval {
    radiance: vec3<f32>,
    transmission: vec3<f32>
}

@group(0) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING: texture_3d<f32>;
@group(0) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(0) @binding(3) var SAMPLER: sampler;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(2) @binding(0) var<uniform> DIRECTION: Direction;
