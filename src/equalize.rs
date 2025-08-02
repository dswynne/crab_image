use ndarray::Array2;

pub fn logeq(data: &Array2<f32>, c: f32) -> Array2<f32> {
    let mut ret = data.clone();
    for ((i, j), value) in data.indexed_iter() {
        ret[[i, j]] = c * (value - 1.0).log10();
    }
    return ret;
}

pub fn powerlaweq(data: &Array2<f32>, c: f32, g: f32) -> Array2<f32> {
    let mut ret = data.clone();
    for ((i, j), value) in data.indexed_iter() {
        ret[[i, j]] = c * value.powf(g);
    }
    return ret;
}
