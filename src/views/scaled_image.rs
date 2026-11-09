//! Local images pre-scaled to the device pixels they cover.
//!
//! GPUI samples images bilinearly without mipmaps, so a 192px avatar drawn at
//! 64 device pixels skips most source pixels and aliases, and a screenshot
//! shrunk to a thumbnail turns its text grainy. Resampling once on the CPU
//! with Lanczos, to exactly the drawn size, keeps edges and detail smooth the
//! way a browser's downscale does.
use gpui::{
    App, Asset, ImageAssetLoader, ImageCacheError, ImageSource, RenderImage, Resource, Window,
};
use image::{Frame, ImageFormat, ImageReader, RgbaImage, imageops::FilterType};
use std::{future::Future, path::PathBuf, sync::Arc};

/// How the image fills its box, matching the `ObjectFit` it is drawn with.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum Fit {
    /// Fills the box, center-cropping the overflow.
    Cover,
    /// Fits inside the box; never enlarged past the source.
    Contain,
}

/// A local image file and the box, in device pixels, to scale it to.
#[derive(Clone, Hash)]
struct ScaledSource {
    path: PathBuf,
    width_px: u32,
    height_px: u32,
    fit: Fit,
}

enum ScaledImageLoader {}

impl Asset for ScaledImageLoader {
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
            let scaled = scale_file(&source)?;
            Ok(Arc::new(RenderImage::new(vec![Frame::new(scaled)])))
        }
    }
}

/// Decodes the file and resamples it for its box, as BGRA for GPUI.
fn scale_file(source: &ScaledSource) -> Result<RgbaImage, ImageCacheError> {
    // Keybase caches attachments as `*.attachment`, so the format comes from
    // the contents, not the extension.
    let reader = ImageReader::open(&source.path)?.with_guessed_format()?;
    if matches!(reader.format(), Some(ImageFormat::Gif | ImageFormat::WebP)) {
        // Possibly animated: leave it to the plain loader, which keeps every frame.
        return Err(ImageCacheError::Asset(
            "animated formats are not pre-scaled".into(),
        ));
    }
    let decoded = reader.decode()?;
    let scaled = match source.fit {
        Fit::Cover => {
            decoded.resize_to_fill(source.width_px, source.height_px, FilterType::Lanczos3)
        }
        Fit::Contain
            if decoded.width() > source.width_px || decoded.height() > source.height_px =>
        {
            decoded.resize(source.width_px, source.height_px, FilterType::Lanczos3)
        }
        // Already small enough: enlarging adds no detail.
        Fit::Contain => decoded,
    };
    let mut scaled = scaled.into_rgba8();
    // GPUI expects BGRA.
    for pixel in scaled.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Ok(scaled)
}

/// An image source for the file at `path` drawn in a `width` by `height`
/// point box, or `None` for a `.gif`, whose frames the plain path loader keeps
/// animating. While scaling is pending, this source stays pending. For a file
/// this loader leaves alone (a GIF or WebP without the extension, or one it
/// cannot decode), the plain loader handles it after scaling returns an error.
pub fn scaled_image_source(
    path: PathBuf,
    width: f32,
    height: f32,
    fit: Fit,
) -> Option<ImageSource> {
    let is_gif = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gif"));
    if is_gif {
        return None;
    }
    let plain = Resource::Path(path.clone().into());
    Some(ImageSource::from(
        move |window: &mut Window, cx: &mut App| {
            let scale = window.scale_factor();
            let source = ScaledSource {
                path: path.clone(),
                width_px: (width * scale).round().max(1.) as u32,
                height_px: (height * scale).round().max(1.) as u32,
                fit,
            };
            match window.use_asset::<ScaledImageLoader>(&source, cx) {
                Some(Ok(image)) => Some(Ok(image)),
                Some(Err(_)) => window.use_asset::<ImageAssetLoader>(&plain, cx),
                None => None,
            }
        },
    ))
}

/// `source` pre-scaled for a `width` by `height` point box when it is a local
/// file; other sources are returned unchanged.
pub fn scaled_for_box(source: ImageSource, width: f32, height: f32, fit: Fit) -> ImageSource {
    match &source {
        ImageSource::Resource(Resource::Path(path)) => {
            scaled_image_source(path.to_path_buf(), width, height, fit).unwrap_or(source)
        }
        _ => source,
    }
}

#[cfg(test)]
mod tests {
    use super::{Fit, ScaledSource, scale_file};
    use image::{ImageFormat, RgbaImage};
    use std::path::{Path, PathBuf};

    /// A `width` by `height` image saved as `format` under a Keybase-style
    /// `.attachment` name, so the extension says nothing about the format.
    fn attachment(name: &str, width: u32, height: u32, format: ImageFormat) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "zbase-scaled-{name}-{}.attachment",
            std::process::id()
        ));
        RgbaImage::new(width, height)
            .save_with_format(&path, format)
            .unwrap();
        path
    }

    fn scaled(path: &Path, width_px: u32, height_px: u32, fit: Fit) -> Option<(u32, u32)> {
        let source = ScaledSource {
            path: path.to_path_buf(),
            width_px,
            height_px,
            fit,
        };
        scale_file(&source).ok().map(|image| image.dimensions())
    }

    #[test]
    fn contain_shrinks_to_the_box_keeping_the_aspect_ratio() {
        let path = attachment("contain", 640, 180, ImageFormat::Png);
        assert_eq!(scaled(&path, 560, 480, Fit::Contain), Some((560, 158)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn contain_never_enlarges() {
        let path = attachment("small", 410, 87, ImageFormat::Png);
        assert_eq!(scaled(&path, 560, 480, Fit::Contain), Some((410, 87)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cover_fills_the_box_exactly() {
        let path = attachment("cover", 640, 480, ImageFormat::Png);
        assert_eq!(scaled(&path, 200, 200, Fit::Cover), Some((200, 200)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn gifs_are_left_to_the_animating_loader() {
        let path = attachment("gif", 64, 64, ImageFormat::Gif);
        assert_eq!(scaled(&path, 32, 32, Fit::Contain), None);
        let _ = std::fs::remove_file(path);
    }
}
