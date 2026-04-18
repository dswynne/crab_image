// External
use ndarray::Array2;
use image::{ImageReader, ImageBuffer, Luma};

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

pub fn array2img(data: &Array2<f32>, file: &str) {
    let height = data.nrows();
    let width = data.ncols();
    let raw: Vec<u8> = data.iter().map(|&x| (x.clamp(0.0, 1.0) * 255.0) as u8).collect();
    let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(width as u32, height as u32, raw).unwrap();
    img.save(file).unwrap();
}

#[allow(dead_code)]
pub fn print_array2(data: &Array2<f32>) {
    for row in data.rows() {
        for val in row {
            print!("{}, ", val);
        }
        println!();
    }
}