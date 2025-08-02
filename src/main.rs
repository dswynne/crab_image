// Standard
use std::env;

// Local
mod equalize;
mod util;

fn main() {
    // Setup
    let cwd = env::current_dir().unwrap();
    let img_arr = util::img2array(cwd.join("data/lena.tif").to_str().unwrap());

    // Run log equalization
    let c: f32 = 1.0;
    let logged = equalize::logeq(&img_arr, c);

    // Print results
    util::print_array2(&logged);
}


