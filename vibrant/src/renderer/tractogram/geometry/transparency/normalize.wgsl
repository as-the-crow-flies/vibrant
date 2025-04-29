@group(0) @binding(3) var COLOR: texture_2d<f32>;

@group(1) @binding(0) var OCCLUSION: texture_3d<f32>;
@group(1) @binding(1) var OCCLUSION_SAMPLER: sampler;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let color = textureLoad(COLOR, vec2<u32>(uv.xy), 0);

    let surface_dim = vec2<f32>(ENVIRONMENT.surface);
    let occlusion_dim = vec2<f32>(textureDimensions(OCCLUSION, 0).xy * ENVIRONMENT.tile);

    let sample = vec3<f32>(uv.x / occlusion_dim.x, 1.0 - uv.y / occlusion_dim.y, 1.0);
    let absorbance = textureSampleLevel(OCCLUSION, OCCLUSION_SAMPLER, sample, 0.0).x;

    let composite = color.rgb / color.a * absorbance;
    // let result = mix(color.rgb, composite, smoothstep(0.0, 1.0, absorbance - color.a));

    let result = select(color.rgb, vec3<f32>(1.0, 0.0, 1.0), absorbance > color.a);

    return vec4<f32>(color.rgb, 1.0);
}
