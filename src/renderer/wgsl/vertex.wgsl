struct Vertex {
    xyz: vec3<f32>,
    clip: vec3<f32>,
}

// `w` packs the clip plane (snorm8 xyz) with an 8-bit hash of the vertex's line.
fn pack_clip_hash(clip: vec3<f32>, hash: u32) -> f32 {
    return bitcast<f32>(pack4x8snorm(vec4<f32>(clip, 0.0)) | (hash << 24u));
}

fn unpack_hash(w: f32) -> u32 {
    return bitcast<u32>(w) >> 24u;
}

fn unpack_vertex(v: vec4<f32>) -> Vertex {
    return Vertex(v.xyz, unpack4x8snorm(bitcast<u32>(v.w)).xyz);
}

fn unpack_vertex_scale(v: vec4<f32>, scale: f32) -> Vertex {
    var vertex = unpack_vertex(v);
    vertex.xyz = (vertex.xyz + 0.5) * scale;
    return vertex;
}
