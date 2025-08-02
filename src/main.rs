// Standard
use std::f64::consts::PI;

// Local
mod equalize;

// External
use ndarray::Array2;


fn main() {
    // Setup
    let mut grid = Array2::<f32>::ones((10, 10));
    grid *= 10.0; // Scale the grid

    // Run log equalization
    let c: f32 = 1.0;
    let logged = equalize::logeq(&grid, c);

    // Print results
    print_array2(&logged);
}

fn print_array2(data: &Array2<f32>) {
    for row in data.rows() {
        for val in row {
            print!("{}, ", val);
        }
        println!();
    }
}
