@group(2) @binding(0) var<storage, read_write> VERTICES: array<u32>;

@group(3) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(3) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

const LOCAL_SORT_SIZE: u32 = 32;

var<private> COLOR: vec4<f32>;

fn visit(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32) -> bool {
    return true;
}

fn result(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    return COLOR;
}
