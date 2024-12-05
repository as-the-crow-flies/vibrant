@group(0) @binding(0) var POSITION: texture_2d<f32>;
@group(0) @binding(1) var NORMAL: texture_2d<f32>;
@group(0) @binding(2) var TANGENT: texture_2d<f32>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Fragment {
    let ndc = 2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0;
    return Fragment(vec4<f32>(ndc, 0.0, 1.0), ndc);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let pixel = vec2<u32>(fragment.clip.xy);

    let position = textureLoad(POSITION, pixel, 0);
    let normal = textureLoad(NORMAL, pixel, 0) * 2.0 - 1.0;
    let tangent = textureLoad(TANGENT, pixel, 0);

    if (position.w == 0.0) { discard; }

    let normal_object_space = normalize(TRACTOGRAM_TO_WORLD * vec4<f32>(normal.xyz, 0.0));
    let tangent_object_space = normalize(TRACTOGRAM_TO_WORLD * vec4<f32>(tangent.xyz, 0.0));

    // return vec4<f32>(vec3<f32>(textureLoad(NORMAL, pixel, 0).x) * vec3<f32>(1.0, 0.0, 1.0), 1.0);

    let boundary = 1.0/3.0;
    let slope = 0.3;
    let func = fragment.ndc.x - slope * fragment.ndc.y;

    if (func < -boundary)
    {
        return vec4<f32>(position.xyz, 1.0);
    }
    else if (func < boundary)
    {
        return vec4<f32>(abs(normal_object_space.xyz), 1.0);
    }
    else
    {
        return vec4<f32>(abs(tangent_object_space.xyz), 1.0);
    }
}
