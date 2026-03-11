struct Material {
    absorption: u32,
    scattering: u32,
};

@group(0) @binding(0) var SEGMENTATION: texture_3d<u32>;
@group(0) @binding(3) var<storage, read> MATERIAL: array<Material>;

@group(1) @binding(0) var ABSORPTION: texture_storage_3d<rgba8unorm, read_write>;
@group(1) @binding(1) var SCATTERING: texture_storage_3d<rgba8unorm, read_write>;
@group(1) @binding(2) var EXTINCTION: texture_storage_3d<rgba8unorm, read_write>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(ABSORPTION))) { return; }

    // let size = vec3<f32>(textureDimensions(ABSORPTION));
    // let min_size = min(min(size.x, size.y), size.z);
    // let midpoint = 0.5 * size;

    // let delta = abs(vec3<f32>(voxel) - midpoint);

    // let sphere = vec3<f32>(0.8 - 0.8 * smoothstep(-0.05, 0.05, (length(delta) / min_size) - 0.25));
    // let axes =  vec3<f32>(0.1 - 0.1 * smoothstep(-0.05, 0.05, min(min(delta.x, delta.y), delta.z) - 1.0));

    // let absorption = sphere;
    // let scattering = sphere;

    let material_id = textureLoad(SEGMENTATION, voxel, 0).x;
    let material = MATERIAL[material_id];

    let absorption = unpack4x8unorm(material.absorption).rgb;
    let scattering = unpack4x8unorm(material.scattering).rgb;

    textureStore(ABSORPTION, voxel, textureLoad(ABSORPTION, voxel) + vec4<f32>(absorption, 0.0));
    textureStore(SCATTERING, voxel, textureLoad(SCATTERING, voxel) + vec4<f32>(scattering, 0.0));
    textureStore(EXTINCTION, voxel, textureLoad(EXTINCTION, voxel) + vec4<f32>(absorption + scattering, 0.0));
}
