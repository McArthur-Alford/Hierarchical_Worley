use std::{
    f32::consts::PI,
    hash::{DefaultHasher, Hash, Hasher},
    time::Instant,
};

use glam::{I8Vec2, IVec2, U8Vec2, U8Vec3, USizeVec2, Vec2, Vec3};
use minifb::{Key, Window, WindowOptions};
use rand::{
    Rng, SeedableRng,
    rngs::{SmallRng, ThreadRng},
};
use rayon::prelude::*;

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

    const SEED: u64 = 10;
    const CELLS: USizeVec2 = USizeVec2::new(4, 4);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        buffer.reset(U8Vec3::ZERO);

        buffer
            .buff
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, pixel)| {
                let x = i % buffer.width;
                let y = i / buffer.width;

                let u = (x as f32 / buffer.width as f32);
                let v = (y as f32 / buffer.height as f32);
                let (cell, dist) = hierarchical_worley((u, v).into(), CELLS, SEED, 4);

                let hash = cell_hash(cell, SEED);
                let mut rng = SmallRng::seed_from_u64(hash);

                let rgb: U8Vec3 = (
                    rng.random_range(0..255),
                    rng.random_range(0..255),
                    rng.random_range(0..255),
                )
                    .into();
                let rgb = (rgb.as_vec3() * (1.0 - dist)).as_u8vec3();

                *pixel = rgb;
            });

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
            let cell_o = (cell.as_ivec2() + IVec2::new(x_o, y_o))
                .clamp(IVec2::ZERO, cells.as_ivec2())
                .as_usizevec2();

            // cell_pos is going to be offset by a cells width and height in the offset direction
            let cell_pos = cell_pos - Vec2::new(x_o as f32, y_o as f32);

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

// Now, for layered worley, its slightly different!
//
//
// Given a pixel pos.
// Determine which cell in our subgrid it belongs to.
// Determine if that cell belongs to our cell? if so, group it. Otherwise not.
//
//
// Given a point (x,y) in [0,1] and columns/rows (C,R) as integers.
// Determine if (x,y) lies within a cell. Return that cell.
//
// Our parent called the above however, on a subcell of its own grid of cells.
// The returned cell becomes a coordinate into our larger grid. We then find which cell
//
//
// Keep calling worley on a point until depth becomes 0.
// Then actually apply worley to get its cell position.
// Return that cell position.
// That cell position becomes the new fractional position, mapped to (0,1)
//
// fn worley(point P, depth):
//  if depth = 0:
//    apply worley as usual. Get a grid cell. Return it.
//  let cell = worley(point, depth-1)
//  convert the cell to a (0,1) float relative to grid[depth-1]
//  apply worley to the cell using grid[depth]. Get a new cell coordinate, return it.
fn hierarchical_worley(
    sample_pos: Vec2,
    cells: USizeVec2,
    seed: u64,
    depth: usize,
) -> (USizeVec2, f32) {
    if depth == 0 {
        return worley(sample_pos, cells, seed);
    }

    let (cell, dist) = hierarchical_worley(sample_pos, cells * 2, seed, depth - 1);
    let sample_pos = cell.as_vec2() / (cells * 2).as_vec2();
    worley(sample_pos, cells, seed)
}
