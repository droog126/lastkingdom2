//! Shared client rendering defaults.

use bevy::image::ImagePlugin;

/// Use linear mip sampling with anisotropy for oblique terrain and GLB
/// surfaces. This preserves detail at a distance without switching the whole
/// game to pixelated nearest-neighbor filtering.
pub(crate) fn crisp_image_plugin() -> ImagePlugin {
    let mut plugin = ImagePlugin::default_linear();
    plugin.default_sampler.set_anisotropic_filter(8);
    plugin
}
