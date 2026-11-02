const INSERTION_SORT_SIZE: u32 = 32;

fn insertion_sort_insert(hits: ptr<function, array<u32, INSERTION_SORT_SIZE>>, hit_count: u32, value: u32) {
    let hit_count_clamped = min(hit_count, INSERTION_SORT_SIZE);
    let idx = insertion_sort_insertion_index(hits, hit_count_clamped, value);

    for (var i = hit_count_clamped; i > idx; i--) {
        (*hits)[i] = (*hits)[i - 1u];
    }

    (*hits)[idx] = value;
}

fn insertion_sort_insertion_index(hits: ptr<function, array<u32, INSERTION_SORT_SIZE>>, hit_count: u32, value: u32) -> u32 {
    var lo: u32 = 0u;
    var hi: u32 = hit_count;

    while (lo < hi) {
        let mid: u32 = (lo + hi) / 2u;
        if ((*hits)[mid] < value) {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }

    return lo;
}
