use std::{cmp::Ordering, collections::BinaryHeap, f32::consts::PI, ops::Add};

use bytemuck::{Pod, Zeroable};
use glam::{UVec2, Vec2, Vec3, Vec4};

/// The HDRI resampled onto Clarberg's equal-area octahedral map, with a mip
/// hierarchy of means. Texels are equal-area, so a node's mean is its average
/// over the sphere and greedy cuts of it partition the sphere exactly.
pub struct Grid {
    levels: Vec<Vec<Texel>>,
}

/// One cell of a cut, as `EnvironmentCell` in uniforms.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct Cell {
    /// The cell's mean direction under the EM weighting.
    direction: [f32; 3],
    /// The cell's solid angle over that of a uniform sample, N·Ω/4π.
    ratio: f32,
    color: [f32; 3],
    /// The cell's average EM weight.
    weight: f32,
}

#[derive(Clone, Copy, Default)]
struct Texel {
    color: Vec3,
    linear: f32,
    log: f32,
    linear_moment: Vec3,
    log_moment: Vec3,
}

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Grid {
    pub const SIZE: u32 = 256;

    pub fn new(size: UVec2, data: &[[f32; 3]]) -> Self {
        let texels = (Self::SIZE * Self::SIZE) as usize;
        let texel_area = 4.0 * PI / texels as f32;
        let mut sum = vec![Texel::default(); texels];

        // Pixels coarser than half a texel are split into k² sub-samples, so
        // every texel gathers several.
        let pixel_angle = (2.0 * PI / size.x as f32).max(PI / size.y as f32);
        let k = (2.0 * pixel_angle / texel_area.sqrt()).ceil().max(1.0) as u32;
        let (columns, rows) = (k * size.x, k * size.y);

        // Every environment carries the energy of the white default: mean
        // luminance 1 over the sphere.
        let mean = mean_luminance(size, data);
        let normalize = if mean > 0.0 { 1.0 / mean } else { 1.0 };

        // Scatter every sub-sample, weighted by its solid angle, into its texel.
        // Dividing by the texel's own area, not the area it happened to gather,
        // keeps the energy exact.
        for row in 0..rows {
            let theta = (row as f32 + 0.5) / rows as f32 * PI;
            let solid_angle = row_solid_angle(row, rows, columns) / texel_area;

            for column in 0..columns {
                let phi = ((column as f32 + 0.5) / columns as f32 - 0.5) * 2.0 * PI;
                let direction = Vec3::new(
                    theta.sin() * phi.cos(),
                    theta.sin() * phi.sin(),
                    theta.cos(),
                );
                let t = Self::index(texel_of(direction));
                let color = pixel(data, size, column / k, row / k) * normalize;

                sum[t] = sum[t] + Texel::new(color, direction).scale(solid_angle);
            }
        }

        let mut levels = vec![sum];

        while levels.last().unwrap().len() > 1 {
            let previous = levels.last().unwrap();
            let size = (Self::SIZE >> levels.len()) as usize;
            let mut level = vec![Texel::default(); size * size];

            for y in 0..size {
                for x in 0..size {
                    let child = |dx, dy| previous[(2 * y + dy) * 2 * size + 2 * x + dx];
                    level[y * size + x] =
                        (child(0, 0) + child(1, 0) + child(0, 1) + child(1, 1)).scale(0.25);
                }
            }

            levels.push(level);
        }

        Self { levels }
    }

    /// N cells for every power-of-two N up to `samples`, at `[N, 2N)`: a greedy
    /// cut by mass under `uniform_share` uniform + the rest by the sampling
    /// weight, so cells are small where the environment is bright.
    pub fn cells(
        &self,
        samples: u32,
        log_samples: bool,
        uniform_share: f32,
        log_em: bool,
    ) -> Vec<Cell> {
        let total = self.levels.last().unwrap()[0].weight(log_samples);
        let share = if total > 0.0 { uniform_share } else { 1.0 };

        let mass = |rect: Rect| {
            let fraction = fraction(rect);
            (1.0 - share) * self.mean(rect).weight(log_samples) * fraction
                / total.max(f32::MIN_POSITIVE)
                + share * fraction
        };

        let mut cells = vec![Cell::default(); 2 * samples as usize];

        for (count, rects) in greedy(samples, mass, |rect| rect.w * rect.h > 1) {
            for (i, rect) in rects.into_iter().enumerate() {
                let mean = self.mean(rect);

                cells[count + i] = Cell {
                    direction: mean
                        .direction(log_em)
                        .unwrap_or_else(|| center(rect))
                        .into(),
                    ratio: count as f32 * fraction(rect),
                    color: mean.color.into(),
                    weight: mean.weight(log_em),
                };
            }
        }

        cells
    }

    /// K seed lobes for every power-of-two K up to `lobes`, at `[K, 2K)`: a
    /// greedy cut by EM-weighted power, each node moment matched by a uniform cap
    /// of its solid angle Ω (r = 1 − Ω/4π).
    pub fn seeds(&self, lobes: u32, log_em: bool) -> Vec<Vec4> {
        // Smallest split whose lobe stays within the κ clamp (U_MIN in gaussian.wgsl).
        const SOLID_ANGLE_MIN: f32 = 1.0 - 1.0 / 1.001;

        let power = |rect: Rect| self.mean(rect).weight(log_em) * fraction(rect);
        let splittable = |rect: Rect| 0.5 * fraction(rect) >= SOLID_ANGLE_MIN;

        let mut seeds = vec![Vec4::ZERO; 2 * lobes as usize];

        for (count, rects) in greedy(lobes, power, splittable) {
            for (i, rect) in rects.into_iter().enumerate() {
                let direction = self
                    .mean(rect)
                    .direction(log_em)
                    .unwrap_or_else(|| center(rect));
                let r = 1.0 - fraction(rect);

                seeds[count + i] = (direction * r).extend(1.0) / count as f32;
            }
        }

        seeds
    }

    // A node is a square or a 2:1 rectangle of level-0 texels: one or two
    // texels at the level of its shorter side.
    fn mean(&self, rect: Rect) -> Texel {
        let m = rect.w.min(rect.h);
        let level = m.trailing_zeros() as usize;
        let size = Self::SIZE >> level;
        let texel = |x: u32, y: u32| self.levels[level][(y * size + x) as usize];

        let (x, y) = (rect.x / m, rect.y / m);
        if rect.w == rect.h {
            return texel(x, y);
        }

        (texel(x, y) + texel(x + rect.w / m - 1, y + rect.h / m - 1)).scale(0.5)
    }

    fn index(texel: UVec2) -> usize {
        (texel.y * Self::SIZE + texel.x) as usize
    }
}

impl Texel {
    fn new(color: Vec3, direction: Vec3) -> Self {
        let linear = luminance(color);
        let log = linear.ln_1p();

        Self {
            color,
            linear,
            log,
            linear_moment: direction * linear,
            log_moment: direction * log,
        }
    }

    fn scale(self, s: f32) -> Self {
        Self {
            color: self.color * s,
            linear: self.linear * s,
            log: self.log * s,
            linear_moment: self.linear_moment * s,
            log_moment: self.log_moment * s,
        }
    }

    fn weight(&self, log: bool) -> f32 {
        if log {
            self.log
        } else {
            self.linear
        }
    }

    fn direction(&self, log: bool) -> Option<Vec3> {
        let moment = if log {
            self.log_moment
        } else {
            self.linear_moment
        };
        (moment.length() > 0.0).then(|| moment.normalize())
    }
}

impl Add for Texel {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            color: self.color + other.color,
            linear: self.linear + other.linear,
            log: self.log + other.log,
            linear_moment: self.linear_moment + other.linear_moment,
            log_moment: self.log_moment + other.log_moment,
        }
    }
}

/// Splits the most massive splittable node in half (alternating x and y) until
/// there are `max`; every power-of-two cut is a prefix of this order.
fn greedy(
    max: u32,
    mass: impl Fn(Rect) -> f32,
    splittable: impl Fn(Rect) -> bool,
) -> Vec<(usize, Vec<Rect>)> {
    let mut heap = BinaryHeap::new();
    let mut done = Vec::new();
    let mut order = 0;

    let mut push = |heap: &mut BinaryHeap<Node>, rect: Rect| {
        heap.push(Node {
            mass: mass(rect),
            order,
            rect,
        });
        order += 1;
    };

    push(
        &mut heap,
        Rect {
            x: 0,
            y: 0,
            w: Grid::SIZE,
            h: Grid::SIZE,
        },
    );

    let snapshot = |heap: &BinaryHeap<Node>, done: &[Rect]| {
        heap.iter()
            .map(|node| node.rect)
            .chain(done.iter().copied())
            .collect::<Vec<_>>()
    };

    let mut cuts = vec![(1, snapshot(&heap, &done))];
    let mut count = 1;

    while count < max as usize {
        let node = loop {
            match heap.pop() {
                Some(node) if splittable(node.rect) => break Some(node),
                Some(node) => done.push(node.rect),
                None => break None,
            }
        };
        let Some(node) = node else { break };

        let (a, b) = split(node.rect);
        push(&mut heap, a);
        push(&mut heap, b);
        count += 1;

        if count.is_power_of_two() {
            cuts.push((count, snapshot(&heap, &done)));
        }
    }

    cuts
}

struct Node {
    mass: f32,
    order: u32,
    rect: Rect,
}

// Most massive first; ties go to the older node.
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.mass
            .total_cmp(&other.mass)
            .then(other.order.cmp(&self.order))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Node {}

fn split(rect: Rect) -> (Rect, Rect) {
    if rect.w == rect.h {
        let w = rect.w / 2;
        (
            Rect { w, ..rect },
            Rect {
                x: rect.x + w,
                w,
                ..rect
            },
        )
    } else {
        let h = rect.h / 2;
        (
            Rect { h, ..rect },
            Rect {
                y: rect.y + h,
                h,
                ..rect
            },
        )
    }
}

/// As a fraction of 4π.
fn fraction(rect: Rect) -> f32 {
    (rect.w * rect.h) as f32 / (Grid::SIZE * Grid::SIZE) as f32
}

fn center(rect: Rect) -> Vec3 {
    let center = Vec2::new(
        rect.x as f32 + 0.5 * rect.w as f32,
        rect.y as f32 + 0.5 * rect.h as f32,
    );
    clarberg(2.0 * center / Grid::SIZE as f32 - 1.0)
}

fn luminance(color: Vec3) -> f32 {
    color.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// Solid-angle weighted, over 4π.
fn mean_luminance(size: UVec2, data: &[[f32; 3]]) -> f32 {
    let mut sum = 0.0f64;

    for y in 0..size.y {
        let solid_angle = row_solid_angle(y, size.y, size.x) as f64;
        for x in 0..size.x {
            sum += solid_angle * luminance(pixel(data, size, x, y)) as f64;
        }
    }

    (sum / (4.0 * PI as f64)) as f32
}

/// Exact solid angle of one pixel in equirect row `y`, not its midpoint
/// estimate, so even a single-row map covers 4π.
fn row_solid_angle(y: u32, rows: u32, columns: u32) -> f32 {
    let theta = |y: u32| y as f32 / rows as f32 * PI;
    2.0 * PI / columns as f32 * (theta(y).cos() - theta(y + 1).cos())
}

// Some maps hold negative or non-finite pixels; neither is light.
fn pixel(data: &[[f32; 3]], size: UVec2, x: u32, y: u32) -> Vec3 {
    let pixel = Vec3::from(data[(y * size.x + x) as usize]);
    Vec3::select(
        pixel.cmpgt(Vec3::ZERO) & pixel.is_finite_mask(),
        pixel,
        Vec3::ZERO,
    )
}

fn texel_of(direction: Vec3) -> UVec2 {
    let uv = clarberg_inverse(direction);
    let texel = ((uv + 1.0) * 0.5 * Grid::SIZE as f32).as_uvec2();
    texel.min(UVec2::splat(Grid::SIZE - 1))
}

/// `clarberg_equal_area_sphere` in sphere.wgsl.
fn clarberg(uv: Vec2) -> Vec3 {
    let (au, av) = (uv.x.abs(), uv.y.abs());

    let d = 1.0 - (au + av);
    let r = 1.0 - d.abs();

    let inv_r = if r > 0.0 { 1.0 / r } else { 0.0 };
    let phi0 = 0.25 * PI * ((av - au) * inv_r + 1.0);
    let xy_scale = r * (2.0 - r * r).max(0.0).sqrt();

    let xy = Vec2::new(sign_not_zero(uv.x), sign_not_zero(uv.y))
        * Vec2::new(phi0.cos(), phi0.sin())
        * xy_scale;
    let z = sign_not_zero(d) * (1.0 - r * r);

    xy.extend(z)
}

fn clarberg_inverse(direction: Vec3) -> Vec2 {
    let r = (1.0 - direction.z.abs()).max(0.0).sqrt();
    let phi0 = direction.y.abs().atan2(direction.x.abs());

    let difference = r * (4.0 * phi0 / PI - 1.0); // av − au
    let sum = if direction.z >= 0.0 { r } else { 2.0 - r }; // au + av

    Vec2::new(
        sign_not_zero(direction.x) * 0.5 * (sum - difference),
        sign_not_zero(direction.y) * 0.5 * (sum + difference),
    )
}

fn sign_not_zero(v: f32) -> f32 {
    if v >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clarberg_round_trips() {
        for i in 0..64 {
            for j in 0..64 {
                let uv = Vec2::new(i as f32 + 0.5, j as f32 + 0.5) / 32.0 - 1.0;
                let back = clarberg_inverse(clarberg(uv));
                assert!((back - uv).length() < 1e-4, "{uv} -> {back}");
            }
        }
    }

    // A sun far beyond the f16 range keeps all its energy, wherever it lands,
    // and every map comes out with the white default's mean luminance.
    #[test]
    fn energy_is_exact() {
        let size = UVec2::new(64, 32);
        let mut data = vec![[0.1; 3]; (size.x * size.y) as usize];
        data[(9 * size.x + 40) as usize] = [1e6; 3];

        for (size, data) in [(size, data), (UVec2::ONE, vec![[3.0; 3]])] {
            let grid = Grid::new(size, &data);
            let mean = grid.levels.last().unwrap()[0].linear;
            assert!((mean - 1.0).abs() < 1e-3, "{size}: {mean}");
        }
    }

    #[test]
    fn cells_partition_the_sphere() {
        let data = vec![[1.0; 3]; 64 * 32];
        let grid = Grid::new(UVec2::new(64, 32), &data);
        let cells = grid.cells(4096, false, 0.1, false);

        for n in [32usize, 128, 256, 1024, 4096] {
            let ratio: f32 = cells[n..2 * n].iter().map(|cell| cell.ratio).sum();
            assert!(
                (ratio / n as f32 - 1.0).abs() < 1e-4,
                "N = {n}: mean ratio {}",
                ratio / n as f32
            );
        }
    }
}
