@group(0) @binding(0) var<storage, read_write> KBUFFER: array<array<u64, SURFACE_X>, SURFACE_Y>;

const U32_MAX: u32 = 4294967295;

@compute
@workgroup_size(WORKGROUP_XY, WORKGROUP_XY)
fn compute(@builtin(global_invocation_id) id: vec3<u32>) {
    KBUFFER[id.y][id.x] = u64(U32_MAX) << 32u;
}
