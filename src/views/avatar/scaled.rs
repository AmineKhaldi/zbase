//! Avatars pre-scaled to the device pixels they cover.
//!
//! GPUI samples images bilinearly without mipmaps, so a 192px avatar drawn at
//! 64 device pixels skips most source pixels and aliases. Resampling once on
//! the CPU with Lanczos, to exactly the drawn size, keeps edges and detail
//! smooth the way a browser's downscale does.
use gpui::{App, Asset, ImageCacheError, ImageSource, RenderImage, Window};
use image::{Frame, imageops::FilterType};
use std::{future::Future, path::PathBuf, sync::Arc};

/// A local image file and the square edge, in device pixels, to scale it to.
#[derive(Clone, Hash)]
struct ScaledSource {
    path: PathBuf,
    edge_px: u32,
}

enum ScaledAvatarLoader {}

impl Asset for ScaledAvatarLoader {
    type Source = ScaledSource;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    #[allow(
        clippy::manual_async_fn,
        reason = "an `async fn` would borrow `App`; the trait needs a 'static future"
    )]
    fn load(
        source: ScaledSource,
        _: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        async move {
            let decoded = image::open(&source.path)?;
            // Center-crops to a square like `ObjectFit::Cover`.
            let mut scaled = decoded
                .resize_to_fill(source.edge_px, source.edge_px, FilterType::Lanczos3)
                .into_rgba8();
            // GPUI expects BGRA.
            for pixel in scaled.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
            Ok(Arc::new(RenderImage::new(vec![Frame::new(scaled)])))
        }
    }
}

/// An image source for the file at `path` drawn as a `size`-point square, or
/// `None` for a GIF, whose frames the plain path loader keeps animating.
pub(super) fn scaled_image_source(path: PathBuf, size: f32) -> Option<ImageSource> {
    let is_gif = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gif"));
    if is_gif {
        return None;
    }
    Some(ImageSource::from(
        move |window: &mut Window, cx: &mut App| {
            let edge_px = (size * window.scale_factor()).round().max(1.) as u32;
            let source = ScaledSource {
                path: path.clone(),
                edge_px,
            };
            window.use_asset::<ScaledAvatarLoader>(&source, cx)
        },
    ))
}
