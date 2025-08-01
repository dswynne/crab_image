// Standard
use std::f64::consts::PI;

// Local
mod equalize;

// External
use image::ImageReader; // TODO: I am confused on why this library no work
use ndarray::{AsArray, arr2};

fn main() {
    // Setup
    // TODO: This should be an image at some point
    let grid: [[f32; 10]; 10] = [[5.0; 10]; 10];
    // let test = AsArray(grid);

    // let a = arr2(&[[1, 2, 3], [4, 5, 6]]);

    // for ((x, y), value) in a.indexed_iter() {
    //     print!("{x},{y},{value}|");
    // }
    let c: f32 = 1.0;

    // Run log equalization
    let mut logged: [[f32; 10]; 10] = equalize::logeq(grid, c);

    // Print results
    for (_i, row) in logged.iter_mut().enumerate() {
        for (_j, col) in row.iter_mut().enumerate() {
            print!("{},", col);
        }
        println!();
    }
}
