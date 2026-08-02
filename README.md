# jixel-image-bindings

[`image`](https://crates.io/crates/image) bindings for the
[`jixel`](https://github.com/awxkee/jixel) JPEG XL encoder.

## Example

```rust
use jixel_image_bindings::JixelEncoder;

fn main() {
    let image = image::open("input.png").unwrap();
    let mut output = std::fs::File::create("output.jxl").unwrap();

    JixelEncoder::new(&mut output)
        .write_dynamic_image(&image)
        .unwrap();
}
```

## License

This project is licensed under either of

- BSD-3-Clause License (see [LICENSE](LICENSE.md))
- Apache License, Version 2.0 (see [LICENSE](LICENSE-APACHE.md))

at your option.
