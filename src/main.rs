use std::{
    f32::consts::PI,
    hash::{DefaultHasher, Hash, Hasher},
    time::Instant,
};

use glam::{I8Vec2, IVec2, U8Vec2, U8Vec3, USizeVec2, Vec2, Vec3};
use image::{Rgb, RgbImage};
use minifb::{Key, Window, WindowOptions};
use rand::{
    Rng, SeedableRng, random,
    rngs::{SmallRng, ThreadRng},
    seq::IndexedRandom,
};
use rand_distr::{Binomial, Distribution};
use rayon::prelude::*;

const WIDTH: usize = 5120;
const HEIGHT: usize = 1440;

#[derive(Clone, Debug)]
pub struct Buffer<T> {
    pub buff: Vec<T>,
    pub width: usize,
    pub height: usize,
}

impl<T: Clone> Buffer<T> {
    pub fn set(&mut self, pos: USizeVec2, val: T) {
        self.buff
            .get_mut(pos.x % self.width + self.width * pos.y)
            .map(|c| *c = val);
    }

    pub fn setf(&mut self, pos: Vec2, val: T) {
        self.set(pos.round().as_usizevec2(), val);
    }

    pub fn get(&mut self, pos: USizeVec2) -> T {
        self.buff
            .get(pos.x % self.width + self.width * pos.y)
            .cloned()
            .unwrap()
    }

    pub fn reset(&mut self, val: T) {
        self.buff = vec![val; self.width * self.height];
    }
}

pub fn rgb_from_u8(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    r << 16 | g << 8 | b
}

pub fn rgb_from_vec(rgb: U8Vec3) -> u32 {
    let (r, g, b) = (rgb.x as u32, rgb.y as u32, rgb.z as u32);
    r << 16 | g << 8 | b
}

fn main() {
    let mut buffer = Buffer {
        width: WIDTH,
        height: HEIGHT,
        buff: vec![U8Vec3::ZERO; WIDTH * HEIGHT],
    };

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(240);
    let mut time = Instant::now();
    let mut refresh = Instant::now();

    let mut seed: u64 = random();
    let mut depth = 8;
    let mut growth = 3.0;
    let mut cells = Vec2::new(256.0, 256.0);
    let mut max_dist = 70.0;
    let mut dist_power = 1.5;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if refresh.elapsed().as_millis() < 1000 {
            // refresh = Instant::now();
            let t = time.elapsed().as_millis() as f32 / 1000.0;
            buffer.reset(U8Vec3::ZERO);

            buffer
                .buff
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, pixel)| {
                    let x = i % buffer.width;
                    let y = i / buffer.width;

                    let (cell, dist) = hierarchical_worley(
                        (x as f32, y as f32).into(),
                        cells,
                        seed,
                        depth,
                        growth,
                    );

                    let hash = cell_hash(cell, seed);
                    let mut rng = SmallRng::seed_from_u64(hash);

                    let rgb: Vec3 = [
                        (255., 167., 0.).into(),
                        (245., 187., 0.).into(),
                        (225., 200., 0.).into(),
                        (255., 85., 85.).into(),
                        (255., 85., 85.).into(),
                        (255., 85., 85.).into(),
                        (49., 0., 62.).into(),
                        (49., 0., 62.).into(),
                        (49., 0., 62.).into(),
                        (49., 0., 62.).into(),
                        (49., 0., 62.).into(),
                        (49., 0., 62.).into(),
                        (82., 7., 130.).into(),
                        (82., 7., 130.).into(),
                        (82., 7., 130.).into(),
                        (82., 7., 130.).into(),
                        (82., 7., 130.).into(),
                        (143., 26., 132.).into(),
                        (143., 26., 132.).into(),
                        (143., 26., 132.).into(),
                        (143., 26., 132.).into(),
                        (143., 26., 132.).into(),
                        (26., 5., 64.).into(),
                        (26., 5., 64.).into(),
                        (26., 5., 64.).into(),
                        (26., 5., 64.).into(),
                        (26., 5., 64.).into(),
                        (80., 250., 123.).into(),
                        (80., 250., 80.).into(),
                        (90., 250., 90.).into(),
                        (80., 250., 60.).into(),
                        (90., 250., 70.).into(),
                        (80., 250., 100.).into(),
                        (98., 114., 164.).into(),
                        // (139., 233., 253.).into(),
                        // (255., 184., 108.).into(),
                        // (255., 121., 198.).into(),
                        // (189., 147., 249.).into(),
                        // (248., 248., 242.).into(),
                        // (40., 42., 54.).into(),
                        // (68., 72., 90.).into(),
                    ]
                    .choose(&mut rng)
                    .cloned()
                    .unwrap();
                    let bin_r = Binomial::new(255, rgb.x as f64 / 255.0).unwrap();
                    let bin_g = Binomial::new(255, rgb.y as f64 / 255.0).unwrap();
                    let bin_b = Binomial::new(255, rgb.z as f64 / 255.0).unwrap();
                    let rgb: U8Vec3 = (
                        bin_r.sample(&mut rng) as u8,
                        bin_g.sample(&mut rng) as u8,
                        bin_b.sample(&mut rng) as u8,
                    )
                        .into();
                    let rgb =
                        (rgb.as_vec3() * (1.0 - dist / max_dist).powf(dist_power)).as_u8vec3();

                    *pixel = rgb;
                });
        }

        window
            .update_with_buffer(
                &buffer
                    .buff
                    .iter()
                    .map(|x| rgb_from_vec(*x))
                    .collect::<Vec<_>>(),
                WIDTH,
                HEIGHT,
            )
            .unwrap();
    }

    let mut img = RgbImage::new(WIDTH as u32, HEIGHT as u32);
    for (i, pixel) in buffer.buff.iter().enumerate() {
        let x = (i % WIDTH) as u32;
        let y = (i / WIDTH) as u32;
        img.put_pixel(x, y, Rgb([pixel.x, pixel.y, pixel.z]));
    }

    img.save("output.png").expect("Failed to save image");
}

// Hashes the seed + cell coordinate
fn cell_hash(cell: IVec2, seed: u64) -> u64 {
    let mut x = (cell.x as i64 as u64).wrapping_mul(0xa0761d6478bd642f);
    let mut y = (cell.y as i64 as u64).wrapping_mul(0xe7037ed1a0b428db);
    let mut s = seed.wrapping_mul(0x8ebc6af09c88c6e3);
    x ^= y.rotate_left(25);
    y ^= s.rotate_left(47);
    s ^= x.rotate_left(17);
    s ^ y
}

// Get the center of a worley cell, ZERO to ONE
fn worley_center(cell: IVec2, seed: u64) -> Vec2 {
    let hash = cell_hash(cell, seed);
    let bits1 = (hash >> 12) as u32;
    let bits2 = (hash >> 32) as u32;
    let x = (bits1 as f32) / (u32::MAX as f32);
    let y = (bits2 as f32) / (u32::MAX as f32);
    (x, y).into()
}

fn worley(sample_pos: Vec2, cell_size: Vec2, seed: u64) -> (IVec2, f32) {
    let pos_in_cells = sample_pos / cell_size;
    let base_cell = pos_in_cells.floor().as_ivec2();

    let mut best_cell = None;
    let mut best_dist = None;

    for xo in -1..=1 {
        for yo in -1..=1 {
            let neighbor = base_cell + IVec2::new(xo, yo);
            let center = worley_center(neighbor, seed);
            let world_center = neighbor.as_vec2() * cell_size + center * cell_size;
            let dist = (world_center - sample_pos).length();

            if best_dist.is_none() || best_dist.unwrap() > dist {
                best_cell = Some(neighbor);
                best_dist = Some(dist);
            }
        }
    }

    (best_cell.unwrap(), best_dist.unwrap())
}

fn hierarchical_worley(
    sample_pos: Vec2,
    cell_size: Vec2,
    seed: u64,
    depth: usize,
    growth: f32,
) -> (IVec2, f32) {
    if depth == 0 {
        let (cell, dist) = worley(sample_pos, cell_size, seed);
        return (cell, 0.0);
    }

    let finer_cell_size = cell_size / growth;
    let (cell, dist) = hierarchical_worley(sample_pos, finer_cell_size, seed, depth - 1, growth);

    let new_sample_pos = cell.as_vec2() * finer_cell_size;
    let (cell_o, dist_o) = worley(new_sample_pos, cell_size, seed);

    (cell_o, dist_o * 0.25 + dist * 0.75)
}
