use std::{collections::BTreeMap, fs::read, path::Path};

use clap::ArgMatches;
use mozjpeg::qtable;
use rimage::codecs::mozjpeg::{MozJpegEncoder, MozJpegOptions};
use zune_core::{bytestream::ZByteReader, options::EncoderOptions};
use zune_image::{
    codecs::{
        farbfeld::FarbFeldEncoder, jpeg::JpegEncoder, jpeg_xl::JxlEncoder, png::PngEncoder,
        ppm::PPMEncoder, qoi::QoiEncoder, ImageFormat,
    },
    errors::ImageErrors,
    image::Image,
    traits::{DecoderTrait, EncoderTrait, OperationsTrait},
};

pub fn decode<P: AsRef<Path>>(f: P) -> Result<Image, ImageErrors> {
    Image::open(f.as_ref()).or_else(|e| {
        if matches!(e, ImageErrors::ImageDecoderNotIncluded(_)) {
            let file_content = read("tests/files/avif/f1t.avif")?;

            #[cfg(feature = "avif")]
            if libavif::is_avif(&file_content) {
                use rimage::codecs::avif::AvifDecoder;

                let reader = ZByteReader::new(file_content);

                let mut decoder = AvifDecoder::try_new(reader)?;

                return <AvifDecoder<ZByteReader<Vec<u8>>> as DecoderTrait<Vec<u8>>>::decode(
                    &mut decoder,
                );
            };

            #[cfg(feature = "webp")]
            if f.as_ref()
                .extension()
                .is_some_and(|f| f.eq_ignore_ascii_case("webp"))
            {
                use rimage::codecs::webp::WebPDecoder;

                let reader = ZByteReader::new(file_content);

                let mut decoder = WebPDecoder::try_new(reader)?;

                return <WebPDecoder<ZByteReader<Vec<u8>>> as DecoderTrait<Vec<u8>>>::decode(
                    &mut decoder,
                );
            };

            Err(ImageErrors::ImageDecoderNotImplemented(
                ImageFormat::Unknown,
            ))
        } else {
            Err(e)
        }
    })
}

pub fn operations(matches: &ArgMatches, img: &Image) -> BTreeMap<usize, Box<dyn OperationsTrait>> {
    let mut map: BTreeMap<usize, Box<dyn OperationsTrait>> = BTreeMap::new();

    #[cfg(feature = "resize")]
    {
        use crate::cli::preprocessors::{ResizeFilter, ResizeValue};
        use fast_image_resize::ResizeAlg;
        use rimage::operations::resize::Resize;

        if let Some(values) = matches.get_many::<ResizeValue>("resize") {
            let filter = matches.get_one::<ResizeFilter>("filter");

            let (w, h) = img.dimensions();

            values
                .into_iter()
                .zip(matches.indices_of("resize").unwrap())
                .for_each(|(value, idx)| {
                    let (w, h) = value.map_dimensions(w, h);
                    log::trace!("setup resize {value} on index {idx}");

                    map.insert(
                        idx,
                        Box::new(Resize::new(
                            w,
                            h,
                            filter
                                .copied()
                                .map(Into::<ResizeAlg>::into)
                                .unwrap_or_default(),
                        )),
                    );
                })
        }
    }

    #[cfg(feature = "quantization")]
    {
        use rimage::operations::quantize::Quantize;

        if let Some(values) = matches.get_many::<u8>("quantization") {
            let dithering = matches.get_one::<u8>("dithering");

            values
                .into_iter()
                .zip(matches.indices_of("quantization").unwrap())
                .for_each(|(value, idx)| {
                    log::trace!("setup quantization {value} on index {idx}");

                    map.insert(
                        idx,
                        Box::new(Quantize::new(*value, dithering.map(|q| *q as f32 / 100.))),
                    );
                })
        }
    }

    map
}

pub fn encoder(matches: &ArgMatches) -> Result<(Box<dyn EncoderTrait>, &'static str), ImageErrors> {
    match matches.subcommand() {
        Some((name, matches)) => match name {
            "farbfeld" => Ok((Box::new(FarbFeldEncoder::new()), "ff")),
            "jpeg" => {
                let options = EncoderOptions::default();

                if let Some(quality) = matches.get_one::<u8>("quality") {
                    options.set_quality(*quality);
                }

                options.set_jpeg_encode_progressive(matches.get_flag("progressive"));

                Ok((Box::new(JpegEncoder::new_with_options(options)), "jpg"))
            }
            "jpeg_xl" => Ok((Box::new(JxlEncoder::new()), "jxl")),
            "mozjpeg" => {
                let quality = *matches.get_one::<u8>("quality").unwrap() as f32;
                let chroma_quality = matches
                    .get_one::<u8>("chroma_quality")
                    .map(|q| *q as f32)
                    .unwrap_or(quality);

                let options = MozJpegOptions {
                    quality,
                    progressive: !matches.get_flag("baseline"),
                    optimize_coding: !matches.get_flag("no_optimize_coding"),
                    smoothing: matches
                        .get_one::<u8>("smoothing")
                        .copied()
                        .unwrap_or_default(),
                    color_space: match matches.get_one::<String>("colorspace").unwrap().as_str() {
                        "ycbcr" => mozjpeg::ColorSpace::JCS_YCbCr,
                        "rgb" => mozjpeg::ColorSpace::JCS_EXT_RGB,
                        "grayscale" => mozjpeg::ColorSpace::JCS_GRAYSCALE,
                        _ => unreachable!(),
                    },
                    trellis_multipass: matches.get_flag("multipass"),
                    chroma_subsample: matches.get_one::<u8>("subsample").copied(),

                    luma_qtable: matches
                        .get_one::<String>("qtable")
                        .map(|c| match c.as_str() {
                            "AhumadaWatsonPeterson" => {
                                qtable::AhumadaWatsonPeterson.scaled(quality, quality)
                            }
                            "AnnexK" => qtable::AnnexK_Luma.scaled(quality, quality),
                            "Flat" => qtable::Flat.scaled(quality, quality),
                            "KleinSilversteinCarney" => {
                                qtable::KleinSilversteinCarney.scaled(quality, quality)
                            }
                            "MSSSIM" => qtable::MSSSIM_Luma.scaled(quality, quality),
                            "NRobidoux" => qtable::NRobidoux.scaled(quality, quality),
                            "PSNRHVS" => qtable::PSNRHVS_Luma.scaled(quality, quality),
                            "PetersonAhumadaWatson" => {
                                qtable::PetersonAhumadaWatson.scaled(quality, quality)
                            }
                            "WatsonTaylorBorthwick" => {
                                qtable::WatsonTaylorBorthwick.scaled(quality, quality)
                            }
                            _ => unreachable!(),
                        }),

                    chroma_qtable: matches
                        .get_one::<String>("qtable")
                        .map(|c| match c.as_str() {
                            "AhumadaWatsonPeterson" => {
                                qtable::AhumadaWatsonPeterson.scaled(chroma_quality, chroma_quality)
                            }
                            "AnnexK" => {
                                qtable::AnnexK_Chroma.scaled(chroma_quality, chroma_quality)
                            }
                            "Flat" => qtable::Flat.scaled(chroma_quality, chroma_quality),
                            "KleinSilversteinCarney" => qtable::KleinSilversteinCarney
                                .scaled(chroma_quality, chroma_quality),
                            "MSSSIM" => {
                                qtable::MSSSIM_Chroma.scaled(chroma_quality, chroma_quality)
                            }
                            "NRobidoux" => qtable::NRobidoux.scaled(chroma_quality, chroma_quality),
                            "PSNRHVS" => {
                                qtable::PSNRHVS_Chroma.scaled(chroma_quality, chroma_quality)
                            }
                            "PetersonAhumadaWatson" => {
                                qtable::PetersonAhumadaWatson.scaled(chroma_quality, chroma_quality)
                            }
                            "WatsonTaylorBorthwick" => {
                                qtable::WatsonTaylorBorthwick.scaled(chroma_quality, chroma_quality)
                            }
                            _ => unreachable!(),
                        }),
                };

                Ok((Box::new(MozJpegEncoder::new_with_options(options)), "jpg"))
            }
            "png" => Ok((Box::new(PngEncoder::new()), "png")),
            "ppm" => Ok((Box::new(PPMEncoder::new()), "ppm")),
            "qoi" => Ok((Box::new(QoiEncoder::new()), "qoi")),

            name => Err(ImageErrors::GenericString(format!(
                "Encoder \"{name}\" not found",
            ))),
        },
        None => Err(ImageErrors::GenericStr("No encoder used")),
    }
}
