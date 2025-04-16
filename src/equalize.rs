// TODO: Can I make this accept variable sized inputs?
pub fn logeq(mut arr: [[f32; 10]; 10], c: f32) -> [[f32; 10]; 10] {
    let mut ret: [[f32; 10]; 10] = [[0.0; 10]; 10];

    for (i, row) in arr.iter_mut().enumerate() {
        for (y, col) in row.iter_mut().enumerate() {
            ret[i][y] = c * (*col - 1.0).log10();
        }
    }

    return ret;
}

// fn power_law() {}
