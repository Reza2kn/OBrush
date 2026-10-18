//! Exact pinned offline LogLuv presets. PNG image uploads mirror Scene.js flipY=true.
//! Encoded channels stay raw; row orientation follows the actual source image upload.
use crate::alpha_decode::decode_alpha_png;
use crate::environment_ibl::EnvironmentAtlas;
use crate::uv_view::{ViewTexture, decode_png};
use std::sync::Arc;
use std::sync::OnceLock;

static UV_PRESET: OnceLock<Result<Arc<ViewTexture>, String>> = OnceLock::new();

pub(crate) fn default_uv_texture() -> Result<Arc<ViewTexture>, String> {
    UV_PRESET
        .get_or_init(|| {
            decode_png("UV grid", include_bytes!("../assets/sculptgl/uv.png")).map(Arc::new)
        })
        .clone()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EnvironmentPreset {
    pub name: &'static str,
    pub png: &'static [u8],
    pub coefficients: [f64; 27],
    pub default_exposure: f32,
}

pub(crate) const PRESETS: [EnvironmentPreset; 5] = [
    EnvironmentPreset {
        name: "Mpumalanga veld",
        png: include_bytes!("../assets/sculptgl/environments/mpumalanga_veld_1k.png"),
        coefficients: [
            0.136819, 0.174125, 0.253762, 0.027778, 0.056838, 0.131221, 0.074356, 0.086793,
            0.099181, -0.07904, -0.091269, -0.102346, -0.02755, -0.0323, -0.039217, 0.031822,
            0.034773, 0.039945, 0.017235, 0.021044, 0.026136, -0.106608, -0.11864, -0.132761,
            0.041, 0.049794, 0.061183,
        ],
        default_exposure: 2.5,
    },
    EnvironmentPreset {
        name: "Venetian crossroads",
        png: include_bytes!("../assets/sculptgl/environments/venetian_crossroads_1k.png"),
        coefficients: [
            0.200626, 0.198426, 0.209579, 0.090452, 0.127807, 0.18839, 0.093188, 0.103245,
            0.106131, 0.033349, 0.054751, 0.067044, 0.07435, 0.08167, 0.079716, 0.063127, 0.08594,
            0.10171, 0.007751, 0.00571, -0.000791, 0.104134, 0.103979, 0.094236, -0.022747,
            -0.028166, -0.037714,
        ],
        default_exposure: 2.5,
    },
    EnvironmentPreset {
        name: "Studio small 01",
        png: include_bytes!("../assets/sculptgl/environments/studio_small_01_1k.png"),
        coefficients: [
            0.534107, 0.589985, 0.617478, 0.119999, 0.13048, 0.128019, 0.089872, 0.088707,
            0.088017, 0.099999, 0.151282, 0.138458, 0.005015, 0.035588, 0.027592, 0.114999,
            0.116739, 0.120579, -0.057997, -0.069532, -0.070401, 0.385123, 0.411714, 0.454725,
            0.303242, 0.333004, 0.35027,
        ],
        default_exposure: 0.5,
    },
    EnvironmentPreset {
        name: "Moonless golf",
        png: include_bytes!("../assets/sculptgl/environments/moonless_golf_1k.png"),
        coefficients: [
            0.137579, 0.112906, 0.09347, 0.070711, 0.066043, 0.065337, -0.029564, -0.02072,
            -0.007737, -0.037254, -0.03327, -0.028294, -0.023847, -0.021208, -0.018767, -0.007873,
            -0.002587, 0.003955, 0.009241, 0.007711, 0.006063, 0.017917, 0.011733, 0.007669,
            0.036859, 0.026285, 0.01474,
        ],
        default_exposure: 1.0,
    },
    EnvironmentPreset {
        name: "Winter river",
        png: include_bytes!("../assets/sculptgl/environments/winter_river_1k.png"),
        coefficients: [
            0.560145, 0.554695, 0.513523, -0.213105, -0.15519, -0.063568, 0.135182, 0.114211,
            0.069349, 0.172852, 0.15182, 0.105477, 0.065753, 0.06405, 0.052622, 0.096352, 0.086557,
            0.063826, 0.02183, 0.01656, 0.008804, 0.186193, 0.16372, 0.119627, 0.025363, 0.022278,
            0.014461,
        ],
        default_exposure: 0.5,
    },
];

#[derive(Default)]
pub(crate) struct EnvironmentLibrary {
    cache: [Option<Arc<EnvironmentAtlas>>; 5],
}

impl EnvironmentLibrary {
    pub(crate) fn get(&mut self, index: u32) -> Result<Arc<EnvironmentAtlas>, String> {
        let index = usize::try_from(index).map_err(|_| "Environment preset index overflow")?;
        let preset = PRESETS.get(index).ok_or("Unknown environment preset")?;
        if let Some(cached) = self.cache[index].as_ref() {
            return Ok(Arc::clone(cached));
        }
        let decoded = decode_alpha_png(preset.png)?;
        if decoded.width != 512
            || decoded.height != 1024
            || preset.png.get(24) != Some(&8)
            || preset.png.get(25) != Some(&6)
        {
            return Err("Exact environment preset requires512x1024 PNG8 RGBA".into());
        }
        // PNG presets use Image in the source, under Scene.js UNPACK_FLIP_Y_WEBGL=true.
        // Typed raw binary atlases are different; these five PNG assets must be row-flipped.
        let row = decoded.width as usize * 4;
        let mut texture_rgba = Vec::with_capacity(decoded.rgba.len());
        for y in (0..decoded.height as usize).rev() {
            texture_rgba.extend_from_slice(&decoded.rgba[y * row..(y + 1) * row]);
        }
        let atlas = Arc::new(EnvironmentAtlas::from_raw(
            decoded.width as usize,
            decoded.height as usize,
            texture_rgba,
            preset.coefficients,
        )?);
        self.cache[index] = Some(Arc::clone(&atlas));
        Ok(atlas)
    }
    pub(crate) fn cached_count(&self) -> usize {
        self.cache.iter().filter(|x| x.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_five_exact_assets_decode_raw_dimensions_and_cache_identity() {
        let mut library = EnvironmentLibrary::default();
        for index in 0..5 {
            let atlas = library.get(index).unwrap();
            assert_eq!(atlas.dimensions(), [512, 1024]);
            assert_eq!(atlas.raw_rgba().len(), 512 * 1024 * 4);
            let decoded = decode_alpha_png(PRESETS[index as usize].png).unwrap();
            for y in [0, 1, 255, 511, 512, 1022, 1023] {
                assert_eq!(
                    &atlas.raw_rgba()[y * 2048..(y + 1) * 2048],
                    &decoded.rgba[(1023 - y) * 2048..(1024 - y) * 2048]
                );
            }
            assert!(Arc::ptr_eq(&atlas, &library.get(index).unwrap()));
            assert_eq!(
                atlas.coefficients().concat(),
                PRESETS[index as usize].coefficients
            );
        }
        assert_eq!(library.cached_count(), 5);
    }
    #[test]
    fn exact_constructor_exposures_and_preset_roster_are_bound() {
        assert_eq!(
            PRESETS.map(|x| x.default_exposure),
            [2.5, 2.5, 0.5, 1.0, 0.5]
        );
        assert_eq!(
            PRESETS.map(|x| x.name),
            [
                "Mpumalanga veld",
                "Venetian crossroads",
                "Studio small 01",
                "Moonless golf",
                "Winter river"
            ]
        );
    }
    #[test]
    fn invalid_preset_preserves_prior_exact_cache() {
        let mut library = EnvironmentLibrary::default();
        let before = library.get(0).unwrap();
        assert!(library.get(5).is_err());
        assert!(library.get(u32::MAX).is_err());
        assert_eq!(library.cached_count(), 1);
        assert!(Arc::ptr_eq(&before, &library.get(0).unwrap()));
    }
}
