from multiprocessing import Pool

import numpy as np

# Spatiotemporal blue noise (Wolfe et al. 2022) by void-and-cluster (Ulichney 1993), as
# seamless Wang-style cells (Cohen et al. 2003): the screen is a checkerboard of SIZE x SIZE
# cells, each picking one of VARIANTS patterns of its colour by hash. A point's energy
# spills into its neighbour cells, averaged over the variants they may hold, so every
# arrangement stays blue across cell edges. Every slice is 2D blue noise and every pixel's
# sequence over the slices is 1D blue noise. Ranks are uniform per cell and slice.
# Output: CELLS x DEPTH x SIZE x SIZE x RGBA, cell = colour * VARIANTS + variant.
SIZE = 22
DEPTH = 32
VARIANTS = 4
CELLS = 2 * VARIANTS
SIGMA = 1.9
DENSITY = 0.1

TEMPORAL = np.exp(-np.minimum(np.arange(DEPTH), DEPTH - np.arange(DEPTH)) ** 2 / (2 * SIGMA ** 2))

# Gaussian over [-SIZE, 2 SIZE) around each position, folded into the cell itself
# (centre) and into its neighbour cells' local coordinates (outer).
_d = np.arange(-SIZE, 2 * SIZE)[None, :] - np.arange(SIZE)[:, None]
_g = np.exp(-_d ** 2 / (2 * SIGMA ** 2))
CENTRE = _g[:, SIZE:2 * SIZE]
OUTER = _g[:, :SIZE] + _g[:, 2 * SIZE:]


def splat(energy, cell, t, y, x, sign):
    cy, cx, oy, ox = CENTRE[y], CENTRE[x], OUTER[y], OUTER[x]
    side = (np.outer(oy, cx) + np.outer(cy, ox)) * (sign / VARIANTS)
    corner = np.outer(oy, ox) * (sign / VARIANTS)

    colour = cell // VARIANTS
    same = slice(colour * VARIANTS, (colour + 1) * VARIANTS)
    other = slice((1 - colour) * VARIANTS, (2 - colour) * VARIANTS)

    energy[cell, t] += sign * np.outer(cy, cx)
    energy[other, t] += side
    energy[same, t] += corner
    energy[cell, :, y, x] += sign * np.roll(TEMPORAL, t)


def blue_noise(seed):
    rng = np.random.default_rng(seed)
    shape = (CELLS, DEPTH, SIZE, SIZE)
    groups = CELLS * DEPTH
    area = SIZE * SIZE

    def unravel(group, local):
        return group // DEPTH, group % DEPTH, local // SIZE, local % SIZE

    def move(energy, pattern, group, local, sign):
        cell, t, y, x = unravel(group, local)
        pattern[cell, t, y, x] = sign > 0
        splat(energy, cell, t, y, x, sign)

    def tightest(energy, pattern, group):
        g = group // DEPTH, group % DEPTH
        return int(np.argmax(np.where(pattern[g], energy[g], -np.inf)))

    def largest(energy, pattern, group):
        g = group // DEPTH, group % DEPTH
        return int(np.argmin(np.where(pattern[g], np.inf, energy[g])))

    # Initial binary pattern, the same count in every cell slice.
    ones = int(DENSITY * area)
    energy = np.zeros(shape)
    pattern = np.zeros(shape, dtype=bool)
    for group in range(groups):
        for local in rng.choice(area, ones, replace=False):
            move(energy, pattern, group, local, 1)

    # Move the tightest cluster to the largest void of its cell slice until stable.
    while True:
        index = int(np.argmax(np.where(pattern, energy, -np.inf)))
        group, local = divmod(index, area)
        move(energy, pattern, group, local, -1)

        void = largest(energy, pattern, group)
        move(energy, pattern, group, void, 1)

        if void == local:
            break

    rank = np.zeros((groups, area), dtype=np.int64)

    # Rank the initial points by removing tightest clusters, in lockstep over cell slices.
    e, p = energy.copy(), pattern.copy()
    for r in range(ones - 1, -1, -1):
        for group in rng.permutation(groups):
            cluster = tightest(e, p, group)
            move(e, p, group, cluster, -1)
            rank[group, cluster] = r

    # Rank the rest by filling the largest voids.
    for r in range(ones, area):
        for group in rng.permutation(groups):
            void = largest(energy, pattern, group)
            move(energy, pattern, group, void, 1)
            rank[group, void] = r

    return (rank * 256 // area).astype(np.uint8).reshape(shape)


if __name__ == '__main__':
    with Pool(4) as pool:
        noise = np.stack(pool.map(blue_noise, range(4)), axis=-1)
    noise.tofile('blue_noise.bin')
