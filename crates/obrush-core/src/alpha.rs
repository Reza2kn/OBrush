//! Bounded, display-independent raster footprints. Image decoding belongs to
//! the host; geometry samples the same grayscale bitmap on every target.

use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::Arc;

pub const MAX_ALPHA_SIDE: u32 = 1024;
pub const MAX_ALPHA_PIXELS: usize = 1024 * 1024;

/// A row-major grayscale alpha. Zero suppresses a brush and 255 keeps its full
/// radial falloff. The first row is the top of the stamp. Cloning shares an
/// immutable bitmap, so queued input does not copy an image for every dab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterAlpha {
    width: u32,
    height: u32,
    pixels: Arc<[u8]>,
    has_signal: bool,
}

impl RasterAlpha {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, String> {
        let count = pixel_count(width, height)?;
        if pixels.len() != count {
            return Err(format!(
                "Alpha dimensions require {count} grayscale pixels, got {}",
                pixels.len()
            ));
        }
        let has_signal = pixels.iter().any(|&pixel| pixel != 0);
        Ok(Self {
            width,
            height,
            pixels: pixels.into(),
            has_signal,
        })
    }

    /// Convert decoded 8-bit RGBA to an alpha. Luminance is measured directly
    /// in the supplied channel space using (77 R + 150 G + 29 B) / 256, then
    /// multiplied by opacity. This is a stamp weight, not a color-space bake.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Self, String> {
        let count = pixel_count(width, height)?;
        if rgba.len() != count * 4 {
            return Err("Decoded alpha RGBA length does not match its dimensions".into());
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate alpha bitmap")?;
        pixels.extend(rgba.chunks_exact(4).map(|pixel| {
            let luminance = (77 * u32::from(pixel[0])
                + 150 * u32::from(pixel[1])
                + 29 * u32::from(pixel[2])
                + 128)
                / 256;
            ((luminance * u32::from(pixel[3]) + 127) / 255) as u8
        }));
        Self::new(width, height, pixels)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn has_signal(&self) -> bool {
        self.has_signal
    }

    /// Bilinear sampling with a zero border. UV (0, 0) is the top-left pixel;
    /// UV (1, 1) is the bottom-right pixel. Invalid or outside UVs yield zero.
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        if !u.is_finite()
            || !v.is_finite()
            || !(0.0..=1.0).contains(&u)
            || !(0.0..=1.0).contains(&v)
        {
            return 0.0;
        }
        let x = u * (self.width - 1) as f32;
        let y = v * (self.height - 1) as f32;
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width as usize - 1);
        let y1 = (y0 + 1).min(self.height as usize - 1);
        let pixel = |x: usize, y: usize| self.pixels[y * self.width as usize + x] as f32 / 255.0;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let top = pixel(x0, y0) + (pixel(x1, y0) - pixel(x0, y0)) * tx;
        let bottom = pixel(x0, y1) + (pixel(x1, y1) - pixel(x0, y1)) * tx;
        top + (bottom - top) * ty
    }
}

fn pixel_count(width: u32, height: u32) -> Result<usize, String> {
    if width == 0 || height == 0 || width > MAX_ALPHA_SIDE || height > MAX_ALPHA_SIDE {
        return Err(format!(
            "Alpha dimensions must be between 1 and {MAX_ALPHA_SIDE} pixels"
        ));
    }
    let count = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= MAX_ALPHA_PIXELS)
        .ok_or("Alpha exceeds the bitmap allocation limit")?;
    Ok(count)
}

impl Serialize for RasterAlpha {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct AlphaRef<'a> {
            width: u32,
            height: u32,
            pixels: &'a [u8],
        }
        AlphaRef {
            width: self.width,
            height: self.height,
            pixels: &self.pixels,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RasterAlpha {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct AlphaInput {
            width: u32,
            height: u32,
            #[serde(deserialize_with = "bounded_pixels")]
            pixels: Vec<u8>,
        }
        let value = AlphaInput::deserialize(deserializer)?;
        Self::new(value.width, value.height, value.pixels).map_err(de::Error::custom)
    }
}

fn bounded_pixels<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    struct PixelsVisitor;
    impl<'de> Visitor<'de> for PixelsVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bounded array of grayscale bytes")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<u8>, A::Error> {
            let mut pixels = Vec::new();
            if sequence
                .size_hint()
                .is_some_and(|size| size > MAX_ALPHA_PIXELS)
            {
                return Err(de::Error::custom(
                    "Alpha exceeds the bitmap allocation limit",
                ));
            }
            while let Some(pixel) = sequence.next_element()? {
                if pixels.len() == MAX_ALPHA_PIXELS {
                    return Err(de::Error::custom(
                        "Alpha exceeds the bitmap allocation limit",
                    ));
                }
                pixels
                    .try_reserve(1)
                    .map_err(|_| de::Error::custom("Cannot allocate alpha bitmap"))?;
                pixels.push(pixel);
            }
            Ok(pixels)
        }
    }
    deserializer.deserialize_seq(PixelsVisitor)
}
