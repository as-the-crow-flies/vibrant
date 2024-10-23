@group(0) @binding(0) var<storage, read_write>  ITEM    : array<array<u32, ITEMS_PER_THREAD>>;
@group(0) @binding(1) var<storage, read_write>  OFFSET  : atomic<u32>;

var<workgroup> WORKGROUP: array<u32, ITEMS_PER_WORKGROUP>;
var<workgroup> WORKGROUP_OFFSET: u32;

fn scan(v: ptr<function, array<u32, ITEMS_PER_THREAD>>) -> u32
{
    var sum = 0u;

    for (var i=0u; i<ITEMS_PER_THREAD; i++)
    {
        let x = (*v)[i];
        (*v)[i] = sum;
        sum += x;
    }

    return sum;
}

fn reduce(global: u32, local: u32) {
    var offset = 1u;
    for (var d = ITEMS_PER_WORKGROUP >> 1u; d > 0u; d >>= 1u) {
        workgroupBarrier();

        if local < d {
            let ai = offset * (2u * local + 1u) - 1u;
            let bi = offset * (2u * local + 2u) - 1u;

            WORKGROUP[bi] += WORKGROUP[ai];
        }

        offset <<= 1u;
    }

    workgroupBarrier();
}

fn downsweep(global: u32, local: u32) {
    var offset = ITEMS_PER_WORKGROUP;
    for (var d = 1u; d < ITEMS_PER_WORKGROUP; d <<= 1u) {
        offset >>= 1u;

        workgroupBarrier();

        if local < d {
            let ai = offset * (2u * local + 1u) - 1u;
            let bi = offset * (2u * local + 2u) - 1u;

            let tmp = WORKGROUP[ai];
            WORKGROUP[ai] = WORKGROUP[bi];
            WORKGROUP[bi] += tmp;
        }
    }

    workgroupBarrier();
}

fn add(v: ptr<function, array<u32, ITEMS_PER_THREAD>>, u: u32)
{
    for (var i=0u; i<ITEMS_PER_THREAD; i++)
    {
        (*v)[i] += u;
    }
}

@compute
@workgroup_size(WORKGROUP_SIZE)
fn compute(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(local_invocation_index) local: u32
) {
    let global = global_invocation_id.x;

    var vec0 = ITEM[2u * global + 0u];
    var vec1 = ITEM[2u * global + 1u];

    WORKGROUP[2u * local + 0u] = scan(&vec0);
    WORKGROUP[2u * local + 1u] = scan(&vec1);

    reduce(global, local);

    if local == 0u {
        // Add workgroup sum to OFFSET buffer & get original index
        WORKGROUP_OFFSET = atomicAdd(&OFFSET, WORKGROUP[ITEMS_PER_WORKGROUP - 1u]);

        // Clear last element
        WORKGROUP[ITEMS_PER_WORKGROUP - 1u] = 0u;
    }

    downsweep(global, local);

    let offset0 = WORKGROUP_OFFSET + WORKGROUP[2u * local + 0u];
    let offset1 = WORKGROUP_OFFSET + WORKGROUP[2u * local + 1u];

    add(&vec0, offset0);
    add(&vec1, offset1);

    ITEM[2u * global + 0u] = vec0;
    ITEM[2u * global + 1u] = vec1;
}
