@group(0) @binding(0) var<storage, read_write>  ITEM    : array<array<ITEM_TYPE, ITEMS_PER_THREAD>>;
@group(0) @binding(1) var<storage, read_write>  SUM     : array<ITEM_TYPE>;

fn add(v: ptr<function, array<ITEM_TYPE, ITEMS_PER_THREAD>>, u: ITEM_TYPE)
{
    for (var i=0u; i<ITEMS_PER_THREAD; i++)
    {
        (*v)[i] += u;
    }
}

@compute
@workgroup_size(WORKGROUP_SIZE)
fn compute(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let global = global_invocation_id.x;

    let sum = SUM[global / WORKGROUP_SIZE];

    var vec0 = ITEM[2u * global + 0u];
    var vec1 = ITEM[2u * global + 1u];

    add(&vec0, sum);
    add(&vec1, sum);

    ITEM[2u * global + 0u] = vec0;
    ITEM[2u * global + 1u] = vec1;
}