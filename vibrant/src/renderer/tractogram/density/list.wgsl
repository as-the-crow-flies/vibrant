// Global Counter to allocate new Linked List Nodes
@group(0) @binding(0) var<storage, read_write> COUNTER: atomic<u32>;

// Per Voxel List Heads
// Expects each head to start at an existing page already
// Can if it is cleared with HEAD[voxel] = global_index << 5u;
@group(0) @binding(1) var<storage, read_write> HEADS: array<atomic<u32>>;

// Unrolled Linked List Page Storage & Pointers to Previous Pages
@group(0) @binding(2) var<storage, read_write> PAGES: array<u32>;
@group(0) @binding(3) var<storage, read_write> PREVIOUS: array<u32>;

let LINE_BITS = 8u;
let SAFETY_BITS + 1u;
let LINE_LENGTH = 1u << LINE_BITS;
let PAGE_SHIFT = LINE_BITS + SAFETY_BITS;

fn store(voxel: u32, data: u32) {
    var head = Head(0u, LINE_LENGTH);

    while (head.line >= LINE_LENGTH)
    {
        head = get_next_line(voxel);

        if (head.line == LINE_LENGTH) {
            allocate_next_page(voxel);
        }
    }

    set_data(head, data);
}

struct Head {
    page: u32,
    line: u32
}

fn get_next_line(voxel: u32) -> Head {
    let head = atomicAdd(&HEADS[voxel], 1u);
    return Head(head >> PAGE_SHIFT, head & (LINE_LENGTH - 1u));
}

fn allocate_next_page(voxel: u32) {
    let next_page = atomicAdd(&COUNTER, 1u);
    PREVIOUS[next_page] = atomicExchange(&HEADS[voxel], next_page << PAGE_SHIFT) >> PAGE_SHIFT;
}

fn set_data(head: Head, data: u32) {
    PAGES[(head.page << LINE_BITS) + head.line] = data;
}
