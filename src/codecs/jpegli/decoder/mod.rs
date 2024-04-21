use std::{io::Read, marker::PhantomData};

use zune_core::{bytestream::ZReaderTrait, colorspace::ColorSpace};
use zune_image::{errors::ImageErrors, image::Image, traits::DecoderTrait};

/// A jpegli decoder
pub struct JpegliDecoder<R: Read> {
    inner: Vec<u8>,
    dimensions: Option<(usize, usize)>,
    phantom: PhantomData<R>,
    colorspace: Option<ColorSpace>,
}

impl<R: Read> JpegliDecoder<R> {
    /// Create a new webp decoder that reads data from `source`
    pub fn try_new(mut source: R) -> Result<JpegliDecoder<R>, ImageErrors> {
        let mut buf = Vec::new();
        source.read_to_end(&mut buf)?;

        Ok(JpegliDecoder {
            inner: buf,
            dimensions: None,
            phantom: PhantomData,
            colorspace: None,
        })
    }
}

impl<R, T> DecoderTrait<T> for JpegliDecoder<R>
where
    R: Read,
    T: ZReaderTrait,
{
    fn decode(&mut self) -> Result<Image, ImageErrors> {
        let image = std::panic::catch_unwind(|| -> Result<Image, std::io::Error> {
            let d = jpegli::Decompress::with_markers(jpegli::ALL_MARKERS).from_mem(&self.inner)?;
            let should_transform_color_space = matches!(
                d.color_space(),
                jpegli::ColorSpace::JCS_EXT_XBGR
                    | jpegli::ColorSpace::JCS_EXT_XRGB
                    | jpegli::ColorSpace::JCS_EXT_ABGR
                    | jpegli::ColorSpace::JCS_RGB565
            );
            let mut image;
            if should_transform_color_space {
                image = d.to_colorspace(jpegli::ColorSpace::JCS_YCbCr)?;
                image.read_scanlines::<rgb::RGB16>()?;
            } else {
                image = d.raw()?;
                image.read_scanlines::<rgb::RGB16>()?;
            }
            let width = image.width().to_owned();
            let height = image.height().to_owned();
            // dimensions = (width, height);
            let colorspace = match image.color_space() {
                jpegli::ColorSpace::JCS_GRAYSCALE => ColorSpace::Luma,
                jpegli::ColorSpace::JCS_RGB => ColorSpace::RGB,
                jpegli::ColorSpace::JCS_YCbCr => ColorSpace::YCbCr,
                jpegli::ColorSpace::JCS_CMYK => ColorSpace::CMYK,
                jpegli::ColorSpace::JCS_YCCK => ColorSpace::YCCK,
                jpegli::ColorSpace::JCS_EXT_RGB => ColorSpace::RGB,
                jpegli::ColorSpace::JCS_EXT_RGBX => ColorSpace::RGBA,
                jpegli::ColorSpace::JCS_EXT_BGR => ColorSpace::BGR,
                jpegli::ColorSpace::JCS_EXT_BGRX => ColorSpace::BGRA,
                jpegli::ColorSpace::JCS_EXT_XBGR => ColorSpace::Unknown,
                jpegli::ColorSpace::JCS_EXT_XRGB => ColorSpace::Unknown,
                jpegli::ColorSpace::JCS_EXT_RGBA => ColorSpace::RGBA,
                jpegli::ColorSpace::JCS_EXT_BGRA => ColorSpace::BGRA,
                jpegli::ColorSpace::JCS_EXT_ABGR => ColorSpace::Unknown,
                jpegli::ColorSpace::JCS_EXT_ARGB => ColorSpace::ARGB,
                jpegli::ColorSpace::JCS_RGB565 => ColorSpace::Unknown,
                jpegli::ColorSpace::JCS_UNKNOWN => ColorSpace::Unknown,
            };
            let pixels = image.finish_into_inner()?;
            Ok(Image::from_u8(pixels, width, height, colorspace))
        })
        .unwrap()
        .map_err(|_| ImageErrors::GenericString("error with jpegli".to_string()))?;
        self.dimensions = Some(image.dimensions());
        self.colorspace = Some(image.colorspace());

        Ok(image)
    }

    fn dimensions(&self) -> Option<(usize, usize)> {
        self.dimensions
    }

    fn out_colorspace(&self) -> ColorSpace {
        self.colorspace.unwrap()
    }

    fn name(&self) -> &'static str {
        "jpegli"
    }
}
