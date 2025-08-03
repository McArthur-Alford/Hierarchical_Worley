use std::{
    f32::consts::PI,
    hash::{DefaultHasher, Hash, Hasher},
    time::Instant,
};

use glam::{I8Vec2, U8Vec2, U8Vec3, USizeVec2, Vec2, Vec3};
use minifb::{Key, Window, WindowOptions};
use rand::{
    Rng, SeedableRng,
    rngs::{SmallRng, ThreadRng},
};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

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
    let time = Instant::now();

    const seed: u64 = 10;
    const cells: USizeVec2 = USizeVec2::new(10, 10);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        buffer.reset(U8Vec3::ZERO);

        for x in 0..buffer.width {
            for y in 0..buffer.height {
                let u = (x as f32 / buffer.width as f32);
                let v = (y as f32 / buffer.height as f32);
                let (cell, dist) = worley((u, v).into(), cells, seed);

                let hash = cell_hash(cell, seed);
                let mut rng = SmallRng::seed_from_u64(hash);

                let rgb: U8Vec3 = (
                    rng.random_range(0..255),
                    rng.random_range(0..255),
                    rng.random_range(0..255),
                )
                    .into();
                let rgb = (rgb.as_vec3() * (1.0 - dist)).as_u8vec3();

                buffer.set((x, y).into(), rgb);
            }
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
}

// Hashes the seed + cell coordinate
fn cell_hash(cell: USizeVec2, seed: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    cell.hash(&mut hasher);
    let hash = hasher.finish();
    hash
}

// Get the center of a worley cell, ZERO to ONE
fn worley_center(cell: USizeVec2, seed: u64) -> Vec2 {
    let hash = cell_hash(cell, seed);
    let mut rng = SmallRng::seed_from_u64(hash);
    (rng.random_range(0.0..1.0), rng.random_range(0.0..1.0)).into()
}

// sample_pos: position (from ZERO to ONE) to sample
// cells: number of cells along x, then y
//
// Returns which cell the given pixel belongs to, and the distance.
fn worley(sample_pos: Vec2, cells: USizeVec2, seed: u64) -> (USizeVec2, f32) {
    let mut best_cell = None;
    let mut best_dist = None;

    // Get our cell
    let cell = (sample_pos * cells.as_vec2())
        .floor()
        .as_usizevec2()
        .min(cells - 1);

    // Map the sample pos to (0,1) within our cell
    let cell_pos = (sample_pos * cells.as_vec2()).fract();

    for x_o in -1..=1 {
        for y_o in -1..=1 {
            let cell_o = (cell.as_i8vec2() + I8Vec2::new(x_o, y_o))
                .clamp(I8Vec2::ZERO, cells.as_i8vec2())
                .as_usizevec2();

            // cell_pos is going to be offset by a cells width and height in the offset direction
            let cell_pos = cell_pos - Vec2::new(x_o.into(), y_o.into());

            // Get the center of our cell
            let center = worley_center(cell_o, seed);

            // Get the distance
            let dist = (center - cell_pos).length();

            if best_dist.is_none() {
                best_cell = Some(cell_o);
                best_dist = Some(dist);
            } else if let Some(old_dist) = best_dist
                && old_dist > dist
            {
                best_cell = Some(cell_o);
                best_dist = Some(dist);
            }
        }
    }

    (best_cell.unwrap(), best_dist.unwrap())
}
