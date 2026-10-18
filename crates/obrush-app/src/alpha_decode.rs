//! Bounded shared PNG/JPEG decoding for brush alphas on native and WASM hosts.
//! Original RGBA bytes remain top-down and retain RGB beneath alpha zero.
use crate::brush_preferences::{MAX_ALPHA_PIXELS, MAX_ALPHA_SIDE};

pub(crate) const MAX_ALPHA_IMAGE_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_ALPHA_DECODER_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct DecodedAlphaImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub(crate) fn decode_alpha_image(bytes: &[u8]) -> Result<DecodedAlphaImage, String> {
    if bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
        decode_alpha_png(bytes)
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        decode_alpha_jpeg(bytes)
    } else {
        Err("Select a valid PNG or JPEG alpha".into())
    }
}

pub(crate) fn decode_alpha_png(bytes: &[u8]) -> Result<DecodedAlphaImage, String> {
    if bytes.len() > MAX_ALPHA_IMAGE_INPUT_BYTES {
        return Err("Alpha PNG exceeds the 16 MiB input limit".into());
    }
    if bytes.len() < 33
        || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10]
        || bytes[8..12] != 13_u32.to_be_bytes()
        || &bytes[12..16] != b"IHDR"
    {
        return Err("Select a valid PNG alpha".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    let rgba_bytes = bounded_rgba_bytes(width, height)
        .ok_or("PNG alpha dimensions must be between 1 and 1024 pixels")?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_ALPHA_SIDE);
    limits.max_image_height = Some(MAX_ALPHA_SIDE);
    limits.max_alloc = Some(MAX_ALPHA_DECODER_BYTES);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| format!("PNG alpha rejected: {error}"))?
        .to_rgba8();
    if decoded.width() != width
        || decoded.height() != height
        || decoded.as_raw().len() != rgba_bytes
    {
        return Err("Decoded PNG dimensions differ from header".into());
    }
    Ok(DecodedAlphaImage {
        width,
        height,
        rgba: decoded.into_raw(),
    })
}

fn decode_alpha_jpeg(bytes: &[u8]) -> Result<DecodedAlphaImage, String> {
    if bytes.len() > MAX_ALPHA_IMAGE_INPUT_BYTES {
        return Err("Alpha JPEG exceeds the 16 MiB input limit".into());
    }
    // Inspect dimensions before allocating the decoded image. The second
    // reader enforces the same bounds while decoding malformed/hostile data.
    let header =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Jpeg);
    let (width, height) = header
        .into_dimensions()
        .map_err(|error| format!("JPEG alpha rejected: {error}"))?;
    let rgba_bytes = bounded_rgba_bytes(width, height)
        .ok_or("JPEG alpha dimensions must be between 1 and 1024 pixels")?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Jpeg);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_ALPHA_SIDE);
    limits.max_image_height = Some(MAX_ALPHA_SIDE);
    limits.max_alloc = Some(MAX_ALPHA_DECODER_BYTES);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| format!("JPEG alpha rejected: {error}"))?
        .to_rgba8();
    if decoded.width() != width
        || decoded.height() != height
        || decoded.as_raw().len() != rgba_bytes
    {
        return Err("Decoded JPEG dimensions differ from header".into());
    }
    Ok(DecodedAlphaImage {
        width,
        height,
        rgba: decoded.into_raw(),
    })
}

fn bounded_rgba_bytes(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 || width > MAX_ALPHA_SIDE || height > MAX_ALPHA_SIDE {
        return None;
    }
    (width as usize)
        .checked_mul(height as usize)
        .filter(|&pixels| pixels <= MAX_ALPHA_PIXELS)?
        .checked_mul(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush_preferences::{AlphaConversion, BrushPreferenceStore};
    use obrush_core::{BrushKind, BrushSettings};
    const HIDDEN: &[u8] = include_bytes!("../tests/fixtures/independent-hidden-rgb2x2.png");
    const JPEG: &[u8] = include_bytes!("../tests/fixtures/independent-gray-bands16x16.jpg");
    const QUANTIZED: &[u8] =
        include_bytes!("../tests/fixtures/independent-alpha-quantization16x16.png");
    const PINNED_QUANTIZED: &[u8] =
        include_bytes!("../tests/fixtures/pinned-sculptgl-alpha-quantization16x16.bin");

    #[test]
    fn independently_encoded_jpeg_decodes_opaque_mean_rgb_on_native_and_wasm() {
        let decoded = decode_alpha_image(JPEG).unwrap();
        assert_eq!([decoded.width, decoded.height], [16, 16]);
        for y in 0..16 {
            for x in 0..16 {
                let at = ((y * 16 + x) * 4) as usize;
                let expected = if x < 8 { 64 } else { 192 };
                assert_eq!(
                    &decoded.rgba[at..at + 4],
                    &[expected, expected, expected, 255]
                );
            }
        }
        let mut store =
            BrushPreferenceStore::from_legacy(&BrushSettings::default(), None, false).unwrap();
        store
            .import_rgba(
                BrushKind::Draw,
                "independent JPEG",
                decoded.width,
                decoded.height,
                &decoded.rgba,
                AlphaConversion::SculptGlMeanRgb,
            )
            .unwrap();
        let alpha = store.library().entries()[0].bitmap();
        assert_eq!(alpha.pixels()[0], 64);
        assert_eq!(alpha.pixels()[15], 192);
    }

    #[test]
    fn malformed_and_oversized_jpeg_reject_before_alpha_installation() {
        for n in [0, 2, 16, 64, JPEG.len() / 2] {
            assert!(decode_alpha_image(&JPEG[..n]).is_err(), "truncation {n}");
        }
        let mut oversized = JPEG.to_vec();
        let marker = oversized
            .windows(2)
            .position(|bytes| bytes == [0xff, 0xc0])
            .expect("independent baseline JPEG uses SOF0");
        oversized[marker + 7..marker + 9].copy_from_slice(&1025_u16.to_be_bytes());
        assert!(
            decode_alpha_image(&oversized)
                .unwrap_err()
                .contains("dimensions")
        );
        assert!(decode_alpha_image(&vec![0xff; MAX_ALPHA_IMAGE_INPUT_BYTES + 1]).is_err());
    }

    #[test]
    fn actual_png_decoder_preserves_top_down_hidden_rgb_and_partial_opacity() {
        let decoded = decode_alpha_png(HIDDEN).unwrap();
        assert_eq!([decoded.width, decoded.height], [2, 2]);
        assert_eq!(
            decoded.rgba,
            [
                255, 0, 0, 0, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 64
            ]
        );
    }

    #[test]
    fn actual_decoded_hidden_rgb_produces_both_conversion_oracles() {
        let rgba = decode_alpha_png(HIDDEN).unwrap();
        for (conversion, expected) in [
            (AlphaConversion::SculptGlMeanRgb, [0, 85, 85, 255]),
            (AlphaConversion::LegacyWeightedOpacity, [0, 75, 29, 64]),
        ] {
            let mut store =
                BrushPreferenceStore::from_legacy(&BrushSettings::default(), None, false).unwrap();
            store
                .import_rgba(
                    BrushKind::Draw,
                    "independent hidden RGB",
                    2,
                    2,
                    &rgba.rgba,
                    conversion,
                )
                .unwrap();
            assert_eq!(store.library().entries()[0].bitmap().pixels(), expected);
        }
    }

    #[test]
    fn partial_opacity_canvas_readback_matches_all_pinned_reference_alpha_bytes() {
        let rgba = decode_alpha_image(QUANTIZED).unwrap();
        assert_eq!([rgba.width, rgba.height], [16, 16]);
        assert_eq!(PINNED_QUANTIZED.len(), 256);
        let mut store =
            BrushPreferenceStore::from_legacy(&BrushSettings::default(), None, false).unwrap();
        store
            .import_rgba(
                BrushKind::Draw,
                "independent alpha quantization",
                rgba.width,
                rgba.height,
                &rgba.rgba,
                AlphaConversion::SculptGlMeanRgb,
            )
            .unwrap();
        assert_eq!(
            store.library().entries()[0].bitmap().pixels(),
            PINNED_QUANTIZED
        );
    }

    #[test]
    fn malformed_and_truncated_pngs_reject() {
        for n in [0, 8, 16, 32, 33, HIDDEN.len() / 2] {
            assert!(decode_alpha_png(&HIDDEN[..n]).is_err(), "truncation {n}");
        }
        let mut signature = HIDDEN.to_vec();
        signature[0] ^= 1;
        assert!(decode_alpha_png(&signature).is_err());
        let mut wrong_ihdr_length = HIDDEN.to_vec();
        wrong_ihdr_length[11] = 12;
        assert!(decode_alpha_png(&wrong_ihdr_length).is_err());
    }

    #[test]
    fn declared_dimensions_and_input_caps_reject_without_decoder_allocation() {
        for [width, height] in [[0, 2], [2, 0], [1025, 2], [2, 1025], [u32::MAX, u32::MAX]] {
            let mut png = HIDDEN.to_vec();
            png[16..20].copy_from_slice(&width.to_be_bytes());
            png[20..24].copy_from_slice(&height.to_be_bytes());
            assert!(decode_alpha_png(&png).is_err());
        }
        assert_eq!(bounded_rgba_bytes(1024, 1024), Some(4 * 1024 * 1024));
        assert_eq!(bounded_rgba_bytes(1, 1024), Some(4096));
        let too_large = vec![0; MAX_ALPHA_IMAGE_INPUT_BYTES + 1];
        assert!(
            decode_alpha_png(&too_large)
                .unwrap_err()
                .contains("input limit")
        );
    }
}
