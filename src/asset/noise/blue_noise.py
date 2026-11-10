from multiprocessing import Pool

import numpy as np

# Spatiotemporal blue noise (Wolfe et al. 2022): void-and-cluster (Ulichney 1993) over a
# DEPTH x SIZE x SIZE volume whose energy is a spatial Gaussian within each slice plus a
# temporal Gaussian along each pixel. Every slice is 2D blue noise and every pixel's
# sequence over the slices is 1D blue noise. One independent volume per RGBA channel.
SIZE = 64
DEPTH = 32
SIGMA = 1.9
DENSITY = 0.1


def gaussian(n):
    d = np.minimum(np.arange(n), n - np.arange(n)).astype(np.float32)
    return np.exp(-d ** 2 / (2 * SIGMA ** 2))


SPATIAL = gaussian(SIZE)[:, None] * gaussian(SIZE)[None, :]
TEMPORAL = gaussian(DEPTH)


def splat(energy, index, sign):
    t, y, x = np.unravel_index(index, energy.shape)
    energy[t] += sign * np.roll(SPATIAL, (y, x), axis=(0, 1))
    energy[:, y, x] += sign * np.roll(TEMPORAL, t)


def blue_noise(seed):
    shape = (DEPTH, SIZE, SIZE)
    n = DEPTH * SIZE * SIZE

    rng = np.random.default_rng(seed)
    pattern = np.zeros(n, dtype=bool)
    pattern[rng.choice(n, int(DENSITY * n), replace=False)] = True

    energy = np.zeros(shape, dtype=np.float32)
    for index in np.flatnonzero(pattern):
        splat(energy, index, 1)

    # Energies masked to the candidates, so each step is a single argmax/argmin.
    clusters = np.where(pattern.reshape(shape), energy, -np.inf)
    voids = np.where(pattern.reshape(shape), np.inf, energy)

    def move(index, sign):
        for e in (energy, clusters, voids):
            splat(e, index, sign)
        clusters.flat[index] = energy.flat[index] if sign > 0 else -np.inf
        voids.flat[index] = np.inf if sign > 0 else energy.flat[index]

    # Initial binary pattern: move the tightest cluster to the largest void until stable.
    while True:
        cluster = np.argmax(clusters)
        move(cluster, -1)

        void = np.argmin(voids)
        move(void, 1)

        if void == cluster:
            break

    rank = np.zeros(n, dtype=np.int64)
    ones = int(np.isfinite(clusters).sum())

    # Rank the initial points by removing tightest clusters.
    c = clusters.copy()
    for r in range(ones - 1, -1, -1):
        cluster = np.argmax(c)
        splat(c, cluster, -1)
        c.flat[cluster] = -np.inf
        rank[cluster] = r

    # Rank the rest by filling the largest voids.
    for r in range(ones, n):
        void = np.argmin(voids)
        splat(voids, void, 1)
        voids.flat[void] = np.inf
        rank[void] = r

    return (rank * 256 // n).astype(np.uint8).reshape(shape)


if __name__ == '__main__':
    with Pool(4) as pool:
        noise = np.stack(pool.map(blue_noise, range(4)), axis=-1)
    noise.tofile('blue_noise.bin')
