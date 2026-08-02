/*
 * // Copyright (c) Radzivon Bartoshyk 8/2026. All rights reserved.
 * //
 * // Redistribution and use in source and binary forms, with or without modification,
 * // are permitted provided that the following conditions are met:
 * //
 * // 1.  Redistributions of source code must retain the above copyright notice, this
 * // list of conditions and the following disclaimer.
 * //
 * // 2.  Redistributions in binary form must reproduce the above copyright notice,
 * // this list of conditions and the following disclaimer in the documentation
 * // and/or other materials provided with the distribution.
 * //
 * // 3.  Neither the name of the copyright holder nor the names of its
 * // contributors may be used to endorse or promote products derived from
 * // this software without specific prior written permission.
 * //
 * // THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * // AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * // IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * // DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * // FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * // DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * // SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * // CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * // OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * // OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

#![forbid(unsafe_code)]

mod alpha;

use std::io::Write;

use alpha::has_alpha;
use image::error::{
    EncodingError, ImageFormatHint, ParameterError, ParameterErrorKind, UnsupportedError,
    UnsupportedErrorKind,
};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageError, ImageResult};

pub use jixel::{
    ColorEncoding, ColorSpace, DarkAqConfig, EncodeConfig, Orientation, Primaries, RenderingIntent,
    Speed, TransferFunction, WhitePoint, distance_from_quality,
};

pub struct JixelEncoder<'a> {
    writer: &'a mut dyn Write,
    pub config: EncodeConfig,
}

fn default_config() -> EncodeConfig {
    EncodeConfig {
        speed: Speed::Slow,
        ..EncodeConfig::default()
    }
    .with_distance(1.25)
}

impl<'a> JixelEncoder<'a> {
    pub fn new(writer: &'a mut dyn Write) -> Self {
        Self::with_config(writer, default_config())
    }

    pub fn with_config(writer: &'a mut dyn Write, config: EncodeConfig) -> Self {
        Self { writer, config }
    }

    pub fn writer(&self) -> &(dyn Write + 'a) {
        self.writer
    }

    pub fn writer_mut(&mut self) -> &mut (dyn Write + 'a) {
        self.writer
    }

    pub fn into_inner(self) -> &'a mut dyn Write {
        self.writer
    }

    /// Encodes a [`DynamicImage`] without converting its pixel format.
    pub fn write_dynamic_image(self, image: &DynamicImage) -> ImageResult<()> {
        ImageEncoder::write_image(
            self,
            image.as_bytes(),
            image.width(),
            image.height(),
            image.color().into(),
        )
    }

    fn encode(
        &self,
        buf: &[u8],
        width: usize,
        height: usize,
        color_type: ExtendedColorType,
    ) -> ImageResult<Vec<u8>> {
        use ExtendedColorType::*;

        let result = match color_type {
            L8 => jixel::encode_image_gray(buf, width, height, &self.config),
            La8 => encode_la8(buf, width, height, &self.config),
            Rgb8 => jixel::encode_image(buf, width, height, &self.config),
            Rgba8 => encode_rgba8(buf, width, height, &self.config),
            L16 => {
                let samples = native_u16(buf);
                jixel::encode_image_gray_16bit(&samples, width, height, &self.config)
            }
            La16 => {
                let samples = native_u16(buf);
                encode_la16(&samples, width, height, &self.config)
            }
            Rgb16 => {
                let samples = native_u16(buf);
                jixel::encode_image_16bit(&samples, width, height, &self.config)
            }
            Rgba16 => {
                let samples = native_u16(buf);
                encode_rgba16(&samples, width, height, &self.config)
            }
            Rgb32F => {
                let samples = native_f32(buf);
                jixel::encode_image_f32(&samples, width, height, &self.config)
            }
            Rgba32F => {
                let samples = native_f32(buf);
                encode_rgba_f32(&samples, width, height, &self.config)
            }
            Bgr8 => {
                let rgb = bgr_to_rgb(buf);
                jixel::encode_image(&rgb, width, height, &self.config)
            }
            Bgra8 => encode_bgra8(buf, width, height, &self.config),
            _ => return Err(unsupported_color(color_type)),
        };

        result.map_err(encoding_error)
    }
}

impl ImageEncoder for JixelEncoder<'_> {
    fn write_image(
        self,
        buf: &[u8],
        width: u32,
        height: u32,
        color_type: ExtendedColorType,
    ) -> ImageResult<()> {
        let bits_per_pixel = u64::from(color_type.bits_per_pixel());
        let row_len = (u64::from(width) * bits_per_pixel).div_ceil(8);
        let expected = row_len.saturating_mul(u64::from(height));
        if expected != buf.len() as u64 {
            return Err(ImageError::Parameter(ParameterError::from_kind(
                ParameterErrorKind::DimensionMismatch,
            )));
        }

        let encoded = self.encode(buf, width as usize, height as usize, color_type)?;
        self.writer.write_all(&encoded).map_err(ImageError::IoError)
    }

    fn set_icc_profile(&mut self, icc_profile: Vec<u8>) -> Result<(), UnsupportedError> {
        self.config.icc_profile = Some(icc_profile);
        Ok(())
    }

    fn set_exif_metadata(&mut self, exif: Vec<u8>) -> Result<(), UnsupportedError> {
        self.config.exif = Some(exif);
        Ok(())
    }
}

fn encode_la8(
    input: &[u8],
    width: usize,
    height: usize,
    config: &EncodeConfig,
) -> Result<Vec<u8>, jixel::EncodeError> {
    if has_alpha::<_, 1, 2>(input, width, width * 2, u8::MAX) {
        jixel::encode_image_gray_alpha(input, width, height, config)
    } else {
        let luma = discard_gray_alpha(input);
        jixel::encode_image_gray(&luma, width, height, config)
    }
}

fn encode_rgba8(
    input: &[u8],
    width: usize,
    height: usize,
    config: &EncodeConfig,
) -> Result<Vec<u8>, jixel::EncodeError> {
    if has_alpha::<_, 3, 4>(input, width, width * 4, u8::MAX) {
        jixel::encode_image_with_alpha(input, width, height, config)
    } else {
        let rgb = discard_rgba_alpha(input);
        jixel::encode_image(&rgb, width, height, config)
    }
}

fn encode_la16(
    input: &[u16],
    width: usize,
    height: usize,
    config: &EncodeConfig,
) -> Result<Vec<u8>, jixel::EncodeError> {
    if has_alpha::<_, 1, 2>(input, width, width * 2, u16::MAX) {
        jixel::encode_image_gray_alpha_16bit(input, width, height, config)
    } else {
        let luma = discard_gray_alpha(input);
        jixel::encode_image_gray_16bit(&luma, width, height, config)
    }
}

fn encode_rgba16(
    input: &[u16],
    width: usize,
    height: usize,
    config: &EncodeConfig,
) -> Result<Vec<u8>, jixel::EncodeError> {
    if has_alpha::<_, 3, 4>(input, width, width * 4, u16::MAX) {
        jixel::encode_image_with_alpha_16bit(input, width, height, config)
    } else {
        let rgb = discard_rgba_alpha(input);
        jixel::encode_image_16bit(&rgb, width, height, config)
    }
}

fn encode_rgba_f32(
    input: &[f32],
    width: usize,
    height: usize,
    config: &EncodeConfig,
) -> Result<Vec<u8>, jixel::EncodeError> {
    if has_alpha::<_, 3, 4>(input, width, width * 4, 1.0) {
        jixel::encode_image_with_alpha_f32(input, width, height, config)
    } else {
        let rgb = discard_rgba_alpha(input);
        jixel::encode_image_f32(&rgb, width, height, config)
    }
}

fn encode_bgra8(
    input: &[u8],
    width: usize,
    height: usize,
    config: &EncodeConfig,
) -> Result<Vec<u8>, jixel::EncodeError> {
    let has_transparency = has_alpha::<_, 3, 4>(input, width, width * 4, u8::MAX);
    let reordered = bgra_to_rgba(input);
    if has_transparency {
        jixel::encode_image_with_alpha(&reordered, width, height, config)
    } else {
        let rgb = discard_rgba_alpha(&reordered);
        jixel::encode_image(&rgb, width, height, config)
    }
}

fn native_u16(buf: &[u8]) -> Vec<u16> {
    buf.as_chunks::<2>()
        .0
        .iter()
        .map(|sample| u16::from_ne_bytes([sample[0], sample[1]]))
        .collect()
}

fn native_f32(buf: &[u8]) -> Vec<f32> {
    buf.as_chunks::<4>()
        .0
        .iter()
        .map(|sample| f32::from_ne_bytes([sample[0], sample[1], sample[2], sample[3]]))
        .collect()
}

fn discard_gray_alpha<T: Copy + Default>(input: &[T]) -> Vec<T> {
    let pixels = input.as_chunks::<2>().0;
    let mut output = vec![T::default(); pixels.len()];
    for (dst, src) in output.iter_mut().zip(pixels) {
        *dst = src[0];
    }
    output
}

fn discard_rgba_alpha<T: Copy + Default>(input: &[T]) -> Vec<T> {
    let pixels = input.as_chunks::<4>().0;
    let mut output = vec![T::default(); pixels.len() * 3];
    for (dst, src) in output.as_chunks_mut::<3>().0.iter_mut().zip(pixels) {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
    }
    output
}

fn bgr_to_rgb(input: &[u8]) -> Vec<u8> {
    let input_pixels = input.as_chunks::<3>().0;
    let mut output = vec![0; input_pixels.len() * 3];
    for (dst, src) in output.as_chunks_mut::<3>().0.iter_mut().zip(input_pixels) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
    }
    output
}

fn bgra_to_rgba(input: &[u8]) -> Vec<u8> {
    let input_pixels = input.as_chunks::<4>().0;
    let mut output = vec![0; input_pixels.len() * 4];
    for (dst, src) in output.as_chunks_mut::<4>().0.iter_mut().zip(input_pixels) {
        dst[0] = src[2];
        dst[1] = src[1];
        dst[2] = src[0];
        dst[3] = src[3];
    }
    output
}

fn format_hint() -> ImageFormatHint {
    ImageFormatHint::Name("JPEG XL".to_owned())
}

fn encoding_error(error: jixel::EncodeError) -> ImageError {
    ImageError::Encoding(EncodingError::new(format_hint(), error))
}

fn unsupported_color(color_type: ExtendedColorType) -> ImageError {
    ImageError::Unsupported(UnsupportedError::from_format_and_kind(
        format_hint(),
        UnsupportedErrorKind::Color(color_type),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_bytes_u16(samples: &[u16]) -> Vec<u8> {
        samples
            .iter()
            .flat_map(|sample| sample.to_ne_bytes())
            .collect()
    }

    #[test]
    fn opaque_la16_is_encoded_as_l16() {
        let la = [100u16, u16::MAX, 200, u16::MAX];
        let expected =
            jixel::encode_image_gray_16bit(&[100, 200], 2, 1, &default_config()).unwrap();
        let mut actual = Vec::new();

        JixelEncoder::new(&mut actual)
            .write_image(&native_bytes_u16(&la), 2, 1, ExtendedColorType::La16)
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn bgr_is_reordered() {
        let mut actual = Vec::new();
        JixelEncoder::new(&mut actual)
            .write_image(&[30, 20, 10], 1, 1, ExtendedColorType::Bgr8)
            .unwrap();
        let expected = jixel::encode_image(&[10, 20, 30], 1, 1, &default_config()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn translucent_bgra_is_reordered_with_alpha() {
        let mut actual = Vec::new();
        JixelEncoder::new(&mut actual)
            .write_image(&[30, 20, 10, 128], 1, 1, ExtendedColorType::Bgra8)
            .unwrap();
        let expected =
            jixel::encode_image_with_alpha(&[10, 20, 30, 128], 1, 1, &default_config()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn image_encoder_metadata_updates_config() {
        let mut output = Vec::new();
        let mut encoder = JixelEncoder::new(&mut output);
        encoder.set_icc_profile(vec![1, 2, 3]).unwrap();
        encoder.set_exif_metadata(vec![4, 5, 6]).unwrap();
        assert_eq!(encoder.config.icc_profile, Some(vec![1, 2, 3]));
        assert_eq!(encoder.config.exif, Some(vec![4, 5, 6]));
    }

    #[test]
    fn unsupported_color_reports_image_error() {
        let mut output = Vec::new();
        let error = JixelEncoder::new(&mut output)
            .write_image(&[0], 1, 1, ExtendedColorType::A8)
            .unwrap_err();
        assert!(matches!(error, ImageError::Unsupported(_)));
    }

    #[test]
    fn invalid_buffer_length_returns_dimension_mismatch() {
        let mut output = Vec::new();
        let error = JixelEncoder::new(&mut output)
            .write_image(&[0, 1], 1, 1, ExtendedColorType::Rgb8)
            .unwrap_err();

        let ImageError::Parameter(error) = error else {
            panic!("expected a parameter error");
        };
        assert_eq!(error.kind(), ParameterErrorKind::DimensionMismatch);
        assert!(output.is_empty());
    }

    #[test]
    fn dynamic_image_is_encoded_directly() {
        let image = DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 128]).unwrap(),
        );
        let mut expected = Vec::new();
        JixelEncoder::new(&mut expected)
            .write_image(
                image.as_bytes(),
                image.width(),
                image.height(),
                image.color().into(),
            )
            .unwrap();

        let mut actual = Vec::new();
        JixelEncoder::new(&mut actual)
            .write_dynamic_image(&image)
            .unwrap();

        assert_eq!(actual, expected);
    }
}
