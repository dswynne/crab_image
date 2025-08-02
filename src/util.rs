// External
use ndarray::Array2;
use image::ImageReader;

pub fn img2array(file: &str) -> Array2<f32> {
    let img = ImageReader::open(file)
        .unwrap()
        .decode()
        .unwrap()
        .to_luma8(); // Convert to 8-bit grayscale

    let (width, height) = img.dimensions();
    let array: Array2<u8> =
        Array2::from_shape_vec((height as usize, width as usize), img.into_raw()).unwrap();

    return array.mapv(|x| x as f32);
}

pub fn print_array2(data: &Array2<f32>) {
    for row in data.rows() {
        for val in row {
            print!("{}, ", val);
        }
        println!();
    }
}