//! Each rendered element owns its image and at most one decode task. Scaled
//! images never enter GPUI's unbounded asset cache or share atlas ownership.
use super::{ScaledSource, scale_file};
use gpui_kit::{
    App, AppContext, Context, ImageCacheError, RenderImage, SharedString, Task, Window,
};
use image::Frame;
use std::sync::Arc;

#[derive(Default)]
struct ImageState {
    completed: Option<ScaledSource>,
    image: Option<Arc<RenderImage>>,
    failed: bool,
    loading: Option<Task<()>>,
}

impl ImageState {
    fn load(&mut self, source: ScaledSource, cx: &mut Context<Self>) {
        if self.loading.is_some() || self.completed.as_ref() == Some(&source) {
            return;
        }
        let decode_source = source.clone();
        let decode = cx.background_spawn(async move {
            scale_file(&decode_source)
                .map(|image| Arc::new(RenderImage::new(vec![Frame::new(image)])))
        });
        self.loading = Some(cx.spawn(async move |this, cx| {
            let result = decode.await;
            let _ = this.update(cx, |state, cx| {
                state.completed = Some(source);
                state.failed = result.is_err();
                if let Ok(image) = result
                    && let Some(old) = state.image.replace(image)
                {
                    cx.drop_image(old, None);
                }
                state.loading = None;
                cx.notify();
            });
        }));
    }
}

pub(super) fn image_for(
    source: ScaledSource,
    window: &mut Window,
    cx: &mut App,
) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
    // The element ID stack distinguishes separate occurrences of the same file.
    // Path and fit also distinguish an element whose content changes in place.
    let key = SharedString::from(format!(
        "scaled-image:{:?}:{}",
        source.fit,
        source.path.display()
    ));
    let state = window.use_keyed_state(key, cx, |_, cx| {
        cx.on_release(|state: &mut ImageState, cx| {
            if let Some(image) = state.image.take() {
                cx.drop_image(image, None);
            }
        })
        .detach();
        ImageState::default()
    });
    state.update(cx, |state, cx| {
        state.load(source.clone(), cx);
        if state.completed.as_ref() == Some(&source) && state.failed {
            Some(Err(ImageCacheError::Asset(
                "image requires the plain loader".into(),
            )))
        } else {
            state.image.clone().map(Ok)
        }
    })
}
