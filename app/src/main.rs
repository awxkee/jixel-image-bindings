use std::fs;
use jixel_image_bindings::JixelEncoder;

fn main() {
    let img = image::open("./assets/digital_art_portrait.jpg").unwrap();
    let mut dst_vec = Vec::new();
    let encoder = JixelEncoder::new(&mut dst_vec);
    encoder.write_dynamic_image(&img).unwrap();
    fs::write("./img.jxl", dst_vec).unwrap();
}
