// Local
mod equalize;

// External
use image::ImageReader;

fn main() {
    // Setup
    // TODO: This should be an image at some point
    let grid: [[f32; 10]; 10] = [[5.0; 10]; 10];
    // TODO: I am confused on why this library no work
    // let img = ImageReader::open("data/lena.tif")?.decode()?;
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
