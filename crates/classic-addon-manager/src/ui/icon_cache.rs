//! Where catalog icons live. The files stay on disk (the HTTP client's [`DiskCache`] in `<cache>/icons`, at most
//! [`DISK_CAP`]); the decoded pixels live in [`IconImages`], which loads an icon only once it is drawn on screen and
//! drops the pixels of icons off screen, longest unseen first, while all decoded icons hold more than
//! [`DECODED_BUDGET`]. A dropped icon that comes back on screen is decoded again from disk.
//!
//! "On screen" is tracked per drawn frame of the app's one window (dialogs are layers of it), which [`frame_marker`]
//! in the shell reports. Icons inside a cached view would need their visibility recorded when the view replays.
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use cam_ui::http_cache::DiskCache;
use cam_ui::http_client;
use gpui_kit::{
    App, AppContext as _, Canvas, Context, Entity, Global, ImageCache, ImageCacheError, ImageCacheItem, ImageSource,
    Img, RenderImage, Resource, SharedString, SharedUri, Styled as _, Window, canvas, img,
};

/// Catalog icons: `https://assets.gaijin.dev/icons/<uuid>.png`.
const CDN_ICONS: &str = "https://assets.gaijin.dev/icons/";
/// Disk space for icon files.
const DISK_CAP: u64 = 128 * 1024 * 1024;
/// Decoded pixels kept while icons are off screen, icons on screen are kept whatever they hold.
const DECODED_BUDGET: usize = 16 * 1024 * 1024;

/// The UUID of a catalog icon URL: exactly [`CDN_ICONS`], a lowercase canonical UUID and `.png`.
pub fn cdn_icon_id(url: &str) -> Option<&str> {
    let id = url.strip_prefix(CDN_ICONS)?.strip_suffix(".png")?;
    let canonical = id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        });
    canonical.then_some(id)
}

/// Installs the HTTP client with the icon files kept in `<cache_dir>/icons`. Call before `cam_ui::init`, which would
/// install it without.
pub fn install_http_client(cx: &mut App, cache_dir: &Path) {
    let cache = DiskCache::new(cache_dir.join("icons"), DISK_CAP, cdn_icon_id);
    http_client::install_with_cache(cx, Some(cache));
}

/// An `img` of `src`. A catalog icon goes through [`IconImages`] and comes with the element that loads it once it is
/// drawn on screen, put both in one `relative` container. Any other URL loads like a plain `img`.
pub fn icon_img(src: SharedString, cx: &mut App) -> (Img, Option<Canvas<()>>) {
    http_client::install(cx);
    if cdn_icon_id(&src).is_none() {
        return (img(src), None);
    }
    let resource = Resource::Uri(SharedUri::from(src));
    let images = IconImages::global(cx);
    let image = img(ImageSource::Resource(resource.clone())).image_cache(&images);
    let shown = canvas(
        move |bounds, window, cx| {
            if bounds.intersects(&window.content_mask().bounds) {
                images.update(cx, |images, cx| images.show(&resource, window, cx));
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
    .inset_0();
    (image, Some(shown))
}

/// Reports every drawn frame to [`IconImages`]. The shell draws one, so icons that left the screen are noticed even
/// when no icon is drawn any more.
pub fn frame_marker() -> Canvas<()> {
    canvas(
        |_, window, cx| {
            if let Some(images) = cx.try_global::<GlobalIconImages>().map(|g| g.0.clone()) {
                images.update(cx, |images, cx| images.frame_drawn(window, cx));
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
}

/// The decoded catalog icons, shared by every catalog icon `img`. Loads start from [`Self::show`] when an icon is
/// drawn on screen, [`ImageCache::load`] only hands out what is loaded or loading, so laying out a row that isn't
/// drawn (the catalog measures every row) doesn't load its icon.
pub struct IconImages {
    entries: HashMap<Resource, Slot>,
    /// Counts drawn frames; `Slot::shown` is the last one an icon was on screen in.
    frame: u64,
    /// Decoded bytes of the loaded entries.
    resident: usize,
    /// Entries still loading, or loaded since the last frame ended.
    unresolved: usize,
    frame_end_scheduled: bool,
}

struct Slot {
    item: ImageCacheItem,
    shown: u64,
    /// Decoded bytes once the load finished, 0 for a failed one.
    bytes: Option<usize>,
}

struct GlobalIconImages(Entity<IconImages>);
impl Global for GlobalIconImages {}

impl IconImages {
    pub fn global(cx: &mut App) -> Entity<Self> {
        if let Some(images) = cx.try_global::<GlobalIconImages>() {
            return images.0.clone();
        }
        let images = cx.new(|_| Self {
            entries: HashMap::new(),
            frame: 0,
            resident: 0,
            unresolved: 0,
            frame_end_scheduled: false,
        });
        cx.set_global(GlobalIconImages(images.clone()));
        images
    }

    /// Marks `resource` as on screen in the frame being drawn and starts loading it when it isn't loaded. Call while
    /// preparing to paint, so the view drawing the icon is redrawn when the load completes.
    fn show(&mut self, resource: &Resource, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(slot) = self.entries.get_mut(resource) {
            slot.shown = self.frame;
            return;
        }
        let item = ImageCacheItem::new(resource, cx);
        item.use_image(window);
        self.entries.insert(resource.clone(), Slot { item, shown: self.frame, bytes: None });
        self.unresolved += 1;
    }

    /// A frame is being drawn: settle the icons after it when loads are pending or the budget is exceeded.
    fn frame_drawn(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.frame_end_scheduled || (self.unresolved == 0 && self.resident <= DECODED_BUDGET) {
            return;
        }
        self.frame_end_scheduled = true;
        let this = cx.entity();
        window.on_next_frame(move |window, cx| this.update(cx, |this, cx| this.end_frame(window, cx)));
    }

    /// After a drawn frame: counts the loads that finished, then drops the decoded pixels of icons not shown in that
    /// frame, longest unseen first, until all fit the budget. Failed loads stay, so a broken icon isn't fetched again
    /// on every draw.
    fn end_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.frame_end_scheduled = false;
        let shown = self.frame;
        self.frame += 1;
        if self.unresolved > 0 {
            for slot in self.entries.values_mut().filter(|slot| slot.bytes.is_none()) {
                let Some(result) = slot.item.get() else { continue };
                let bytes = result.map_or(0, |image| decoded_len(&image));
                slot.bytes = Some(bytes);
                self.resident += bytes;
                self.unresolved -= 1;
            }
        }
        if self.resident <= DECODED_BUDGET {
            return;
        }
        let mut idle: Vec<(u64, Resource)> = self
            .entries
            .iter()
            .filter(|(_, slot)| slot.shown != shown && slot.bytes.is_some_and(|bytes| bytes > 0))
            .map(|(resource, slot)| (slot.shown, resource.clone()))
            .collect();
        idle.sort_unstable_by_key(|&(shown, _)| shown);
        for (_, resource) in idle {
            if self.resident <= DECODED_BUDGET {
                break;
            }
            let Some(slot) = self.entries.remove(&resource) else { continue };
            self.resident -= slot.bytes.unwrap_or(0);
            if let Some(Ok(image)) = slot.item.get() {
                cx.drop_image(image, Some(window));
            }
        }
    }
}

/// Bytes of decoded pixels in `image`, all frames.
fn decoded_len(image: &RenderImage) -> usize {
    (0..image.frame_count()).filter_map(|i| image.as_bytes(i)).map(<[u8]>::len).sum()
}

impl ImageCache for IconImages {
    fn load(
        &mut self,
        resource: &Resource,
        window: &mut Window,
        _cx: &mut App,
    ) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        self.entries.get(resource)?.item.use_image(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_canonical_cdn_icon_urls_have_an_id() {
        let id = "23f289ab-6aaf-495a-8bf4-0ed953bcb6c7";
        assert_eq!(cdn_icon_id(&format!("{CDN_ICONS}{id}.png")), Some(id));
        for url in [
            format!("{CDN_ICONS}{}.png", id.to_uppercase()),
            format!("{CDN_ICONS}{id}.png?v=2"),
            format!("{CDN_ICONS}{id}.jpg"),
            format!("{CDN_ICONS}x/{id}.png"),
            format!("{CDN_ICONS}{}.png", id.replace('-', "")),
            format!("http://assets.gaijin.dev/icons/{id}.png"),
            format!("https://raw.githubusercontent.com/a/b/main/{id}.png"),
        ] {
            assert_eq!(cdn_icon_id(&url), None, "{url}");
        }
    }
}
