//! Scaling a picked icon down before it is uploaded. Icons are drawn at 56 px at most, but the files developers pick
//! are often thousands of pixels wide, and every user of the catalog decodes the stored file.
use std::io::Cursor;

use image::{
    DynamicImage, ImageError, Limits, Rgba, RgbaImage,
    codecs::png::{CompressionType, FilterType as PngFilter, PngDecoder, PngEncoder},
    imageops::FilterType,
};

use super::png_dimensions;

/// Longest edge of an uploaded icon. The largest icon drawn is 56 px: 112 physical pixels at 200 % scaling.
const ICON_EDGE: u32 = 128;
/// Longest source edge accepted. The source is decoded in one piece: 64 MiB of RGBA at 4096×4096.
pub(super) const ICON_MAX_DIMENSION: u32 = 4096;
/// Lanczos3 widens its kernel with the scale factor, so filtering a 4096 px source down to 128 px reads about 190
/// source pixels per output pixel and pass. A larger source is first box-averaged to this multiple of the target,
/// which reads every source pixel once, and Lanczos3 does only the last step.
const PRESCALE: u32 = 2;

/// The PNG to upload for the icon file `data`: `data` itself when it fits in [`ICON_EDGE`], otherwise the image scaled
/// down to fit (aspect ratio kept) and encoded again. Sources larger than [`ICON_MAX_DIMENSION`] are an
/// [`ImageError::Limits`] before any pixel is decoded.
pub(super) fn prepare_icon(data: Vec<u8>) -> Result<Vec<u8>, ImageError> {
    if png_dimensions(&data).is_some_and(|(width, height)| width.max(height) <= ICON_EDGE) {
        return Ok(data);
    }
    let image = decode(&data)?;
    // Only the pixels are needed from here on: free the file before the scaled copies are allocated.
    drop(data);
    encode(&shrink(image))
}

fn decode(data: &[u8]) -> Result<DynamicImage, ImageError> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(ICON_MAX_DIMENSION);
    limits.max_image_height = Some(ICON_MAX_DIMENSION);
    DynamicImage::from_decoder(PngDecoder::with_limits(Cursor::new(data), limits)?)
}

/// `image` scaled to fit in [`ICON_EDGE`]. The source is dropped as soon as it has been sampled.
fn shrink(image: DynamicImage) -> DynamicImage {
    let (width, height) = fit(image.width(), image.height());
    if !image.color().has_alpha() {
        return resample(&image, width, height);
    }
    // Filters average neighboring pixels. On straight alpha the color of transparent pixels, usually black or white,
    // bleeds into every soft edge as a dark or light fringe, on premultiplied alpha it weighs nothing.
    let mut rgba = image.into_rgba8();
    premultiply(&mut rgba);
    let mut small = resample(&DynamicImage::ImageRgba8(rgba), width, height).into_rgba8();
    unpremultiply(&mut small);
    DynamicImage::ImageRgba8(small)
}

/// The size of a `width`×`height` image scaled so its longest edge is [`ICON_EDGE`], each edge at least 1 px.
fn fit(width: u32, height: u32) -> (u32, u32) {
    let long = u64::from(width.max(height));
    let scale = |edge: u32| ((u64::from(edge) * u64::from(ICON_EDGE) + long / 2) / long).max(1) as u32;
    (scale(width), scale(height))
}

fn resample(image: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    let (pre_width, pre_height) = (width * PRESCALE, height * PRESCALE);
    if image.width() > pre_width && image.height() > pre_height {
        image.thumbnail_exact(pre_width, pre_height).resize_exact(width, height, FilterType::Lanczos3)
    } else {
        image.resize_exact(width, height, FilterType::Lanczos3)
    }
}

fn premultiply(image: &mut RgbaImage) {
    for Rgba([r, g, b, a]) in image.pixels_mut() {
        let alpha = u16::from(*a);
        if alpha < 255 {
            for channel in [r, g, b] {
                *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
    }
}

fn unpremultiply(image: &mut RgbaImage) {
    for Rgba([r, g, b, a]) in image.pixels_mut() {
        let alpha = u16::from(*a);
        match alpha {
            255 => {}
            // Fully transparent: the color is invisible, zero compresses best.
            0 => [*r, *g, *b] = [0; 3],
            // Lanczos3 overshoots near hard edges, so a channel can come out above its alpha.
            _ => {
                for channel in [r, g, b] {
                    *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
    }
}

fn encode(image: &DynamicImage) -> Result<Vec<u8>, ImageError> {
    let mut png = Vec::new();
    image.write_with_encoder(PngEncoder::new_with_quality(&mut png, CompressionType::Best, PngFilter::Adaptive))?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use image::ImageFormat;

    use super::*;

    fn png_of(image: impl Into<DynamicImage>) -> Vec<u8> {
        let mut png = Cursor::new(Vec::new());
        image.into().write_to(&mut png, ImageFormat::Png).unwrap();
        png.into_inner()
    }

    fn decoded(png: &[u8]) -> RgbaImage {
        image::load_from_memory_with_format(png, ImageFormat::Png).unwrap().into_rgba8()
    }

    #[test]
    fn large_icons_are_scaled_to_fit_the_icon_edge() {
        for ((width, height), expected) in [((512, 512), (128, 128)), ((1000, 500), (128, 64)), ((4096, 2), (128, 1))] {
            let source = png_of(RgbaImage::from_pixel(width, height, Rgba([10, 20, 30, 255])));
            let icon = decoded(&prepare_icon(source).unwrap());
            assert_eq!(icon.dimensions(), expected, "source {width}×{height}");
        }
    }

    #[test]
    fn icons_that_already_fit_are_uploaded_unchanged() {
        for edge in [50, 128] {
            let source = png_of(RgbaImage::from_pixel(edge, edge, Rgba([1, 2, 3, 4])));
            assert_eq!(prepare_icon(source.clone()).unwrap(), source);
        }
    }

    #[test]
    fn opaque_rgb_icons_keep_their_colour() {
        let source = png_of(image::RgbImage::from_pixel(600, 600, image::Rgb([200, 100, 50])));
        let icon = decoded(&prepare_icon(source).unwrap());
        assert!(icon.pixels().all(|p| p.0 == [200, 100, 50, 255]));
    }

    #[test]
    fn transparent_pixels_do_not_darken_soft_edges() {
        // Opaque red on the left half, transparent black on the right: every pixel the scaled edge keeps visible is
        // pure red, only its alpha falls off.
        let source = RgbaImage::from_fn(1024, 1024, |x, _| if x < 512 { Rgba([255, 0, 0, 255]) } else { Rgba([0; 4]) });
        let icon = decoded(&prepare_icon(png_of(source)).unwrap());
        let edge: Vec<_> = icon.pixels().filter(|p| p[3] > 0 && p[3] < 255).collect();
        assert!(!edge.is_empty());
        assert!(edge.iter().all(|p| p[0] >= 250 && p[1] == 0 && p[2] == 0), "{edge:?}");
    }

    #[test]
    fn sources_beyond_the_max_dimension_are_refused_before_decoding() {
        let source = png_of(RgbaImage::new(ICON_MAX_DIMENSION + 1, 1));
        assert!(matches!(prepare_icon(source), Err(ImageError::Limits(_))));
    }

    #[test]
    fn data_that_is_not_a_png_is_an_error() {
        assert!(matches!(prepare_icon(b"\xFF\xD8\xFF\xE0 a jpeg".to_vec()), Err(ImageError::Decoding(_))));
    }
}
