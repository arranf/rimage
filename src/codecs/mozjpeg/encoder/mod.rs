use std::mem;

use mozjpeg::qtable::QTable;
use zune_core::{bit_depth::BitDepth, colorspace::ColorSpace};
use zune_image::{codecs::ImageFormat, errors::ImageErrors, image::Image, traits::EncoderTrait};

/// Advanced options for MozJpeg encoding
pub struct MozJpegOptions {
    quality: f32,
    progressive: bool,
    optimize_coding: bool,
    smoothing: u8,
    color_space: mozjpeg::ColorSpace,
    trellis_multipass: bool,
    chroma_subsample: Option<u8>,
    luma_qtable: Option<&'static QTable>,
    chroma_qtable: Option<&'static QTable>,
}

/// A MozJpeg encoder
pub struct MozJpegEncoder {
    options: MozJpegOptions,
}

impl Default for MozJpegOptions {
    fn default() -> Self {
        Self {
            quality: 75.,
            progressive: true,
            optimize_coding: true,
            smoothing: 0,
            color_space: mozjpeg::ColorSpace::JCS_YCbCr,
            trellis_multipass: false,
            chroma_subsample: None,
            luma_qtable: None,
            chroma_qtable: None,
        }
    }
}

impl Default for MozJpegEncoder {
    fn default() -> Self {
        Self {
            options: Default::default(),
        }
    }
}

impl MozJpegEncoder {
    /// Create a new encoder
    pub fn new() -> MozJpegEncoder {
        MozJpegEncoder::default()
    }

    /// Create a new encoder with specified options
    pub fn new_with_options(options: MozJpegOptions) -> MozJpegEncoder {
        MozJpegEncoder { options }
    }
}

impl EncoderTrait for MozJpegEncoder {
    fn name(&self) -> &'static str {
        "mozjpeg-encoder"
    }

    fn encode_inner(&mut self, image: &Image) -> Result<Vec<u8>, ImageErrors> {
        let (width, height) = image.dimensions();
        let data = &image.flatten_to_u8()[0];

        std::panic::catch_unwind(|| -> Result<Vec<u8>, ImageErrors> {
            let format = match image.colorspace() {
                ColorSpace::RGB => mozjpeg::ColorSpace::JCS_RGB,
                ColorSpace::RGBA => mozjpeg::ColorSpace::JCS_EXT_RGBA,
                ColorSpace::YCbCr => mozjpeg::ColorSpace::JCS_YCbCr,
                ColorSpace::Luma => mozjpeg::ColorSpace::JCS_GRAYSCALE,
                ColorSpace::YCCK => mozjpeg::ColorSpace::JCS_YCCK,
                ColorSpace::CMYK => mozjpeg::ColorSpace::JCS_CMYK,
                ColorSpace::BGR => mozjpeg::ColorSpace::JCS_EXT_BGR,
                ColorSpace::BGRA => mozjpeg::ColorSpace::JCS_EXT_BGRA,
                ColorSpace::ARGB => mozjpeg::ColorSpace::JCS_EXT_ARGB,
                ColorSpace::Unknown => mozjpeg::ColorSpace::JCS_UNKNOWN,
                _ => mozjpeg::ColorSpace::JCS_UNKNOWN,
            };

            let mut comp = mozjpeg::Compress::new(format);

            comp.set_size(width, height);
            comp.set_quality(self.options.quality);

            if self.options.progressive {
                comp.set_progressive_mode();
            }

            comp.set_optimize_coding(self.options.optimize_coding);
            comp.set_smoothing_factor(self.options.smoothing);
            comp.set_color_space(match format {
                mozjpeg::ColorSpace::JCS_GRAYSCALE => {
                    log::warn!("Input colorspace is GRAYSCALE, using GRAYSCALE as output");

                    mozjpeg::ColorSpace::JCS_GRAYSCALE
                }
                mozjpeg::ColorSpace::JCS_CMYK => {
                    log::warn!("Input colorspace is CMYK, using CMYK as output");

                    mozjpeg::ColorSpace::JCS_CMYK
                }
                mozjpeg::ColorSpace::JCS_YCCK => {
                    log::warn!("Input colorspace is YCCK, using YCCK as output");

                    mozjpeg::ColorSpace::JCS_YCCK
                }

                _ => self.options.color_space,
            });
            comp.set_use_scans_in_trellis(self.options.trellis_multipass);

            if let Some(sb) = self.options.chroma_subsample {
                comp.set_chroma_sampling_pixel_sizes((sb, sb), (sb, sb))
            }

            if let Some(qtable) = self.options.luma_qtable {
                comp.set_luma_qtable(qtable)
            }

            if let Some(qtable) = self.options.chroma_qtable {
                comp.set_chroma_qtable(qtable)
            }

            let mut comp = comp.start_compress(Vec::new())?;

            comp.write_scanlines(&data)?;

            Ok(comp.finish()?)
        })
        .map_err(|err| {
            if let Ok(mut err) = err.downcast::<String>() {
                ImageErrors::EncodeErrors(zune_image::errors::ImgEncodeErrors::Generic(mem::take(
                    &mut *err,
                )))
            } else {
                ImageErrors::EncodeErrors(zune_image::errors::ImgEncodeErrors::GenericStatic(
                    "Unknown error occurred during encoding",
                ))
            }
        })?
    }

    fn supported_colorspaces(&self) -> &'static [ColorSpace] {
        &[
            ColorSpace::Luma,
            ColorSpace::RGBA,
            ColorSpace::RGB,
            ColorSpace::YCCK,
            ColorSpace::CMYK,
            ColorSpace::BGR,
            ColorSpace::BGRA,
            ColorSpace::ARGB,
            ColorSpace::YCbCr,
        ]
    }

    fn format(&self) -> zune_image::codecs::ImageFormat {
        ImageFormat::JPEG
    }

    fn supported_bit_depth(&self) -> &'static [BitDepth] {
        &[BitDepth::Eight, BitDepth::Sixteen]
    }

    fn default_depth(&self, depth: BitDepth) -> BitDepth {
        match depth {
            BitDepth::Sixteen | BitDepth::Float32 => BitDepth::Sixteen,
            _ => BitDepth::Eight,
        }
    }
}

#[cfg(test)]
mod tests;
