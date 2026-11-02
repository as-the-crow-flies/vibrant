struct Vertex {
    xyz: vec3<f32>,
    clip: vec3<f32>,
    alpha: f32
}

fn pack_clip_alpha(clip_alpha: vec4<f32>) -> f32 {
    let clip = pack4x8snorm(vec4<f32>(clip_alpha.xyz, 0.0));
    let alpha = pack4x8unorm(vec4<f32>(vec3<f32>(0.0), clip_alpha.a));

    return bitcast<f32>(clip | alpha);
}

fn unpack_clip_alpha(f: f32) -> vec4<f32> {
    let u = bitcast<u32>(f);

    let clip = unpack4x8snorm(u);
    let alpha = unpack4x8unorm(u);

    return vec4<f32>(clip.xyz, alpha.a);
}

fn unpack_vertex(v: vec4<f32>) -> Vertex {
    let clip_alpha = unpack_clip_alpha(v.a);
    return Vertex(v.xyz, clip_alpha.xyz, clip_alpha.a);
}

fn unpack_vertex_scale(v: vec4<f32>, scale: f32) -> Vertex {
    var vertex = unpack_vertex(v);
    vertex.xyz = (vertex.xyz + 0.5) * scale;
    return vertex;
}

fn should_be_clipped(v0: Vertex, v1: Vertex, position: vec3<f32>) -> bool {
    return dot(position - v0.xyz, v0.clip) < 0.0 || dot(v1.xyz - position, v1.clip) < 0.0;
}
