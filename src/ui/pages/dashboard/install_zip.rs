//! Install addons from ZIP files: pick one with the native file picker or take dropped files, install them through the
//! addon store and toast each outcome.
use std::path::PathBuf;

use gpui_kit::{App, PathPromptOptions};

use crate::{
    state::{self, tasks::error_message},
    ui::{icons, toast},
};

/// Opens the native file picker and installs the chosen archive.
pub fn pick_and_install(cx: &mut App) {
    let picked = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Install".into()),
    });
    cx.spawn(async move |cx| {
        // The picker is cancelled (no file) or fails (portal missing): nothing to do either way.
        let Ok(Ok(Some(paths))) = picked.await else { return };
        let Some(path) = paths.into_iter().next() else { return };
        cx.update(|cx| install_paths(vec![path], cx));
    })
    .detach();
}

/// Installs every given archive. The store installs them one after the other, after any archives given before.
pub fn install_paths(paths: Vec<PathBuf>, cx: &mut App) {
    let store = state::addons(cx);
    let installs: Vec<_> = paths.into_iter().map(|path| store.update(cx, |s, cx| s.install_zip(path, cx))).collect();
    cx.spawn(async move |cx| {
        for install in installs {
            let result = install.await;
            cx.update(|cx| match result {
                Ok(name) => {
                    toast::show(cx, toast::Toast::new("Addon Installed", format!("{name} installed successfully!")));
                }
                Err(e) => {
                    let message = error_message(&e);
                    if !message.contains("shellItem is nil") {
                        toast::show(cx, toast::Toast::new("Error", message).icon(icons::TRIANGLE_ALERT));
                    }
                }
            });
        }
    })
    .detach();
}

/// Keeps only `.zip` files of a drop (case-insensitive).
pub fn zip_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"))).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_zip_files() {
        let paths = vec![
            PathBuf::from("/a/b.zip"),
            PathBuf::from("/a/c.ZIP"),
            PathBuf::from("/a/d.txt"),
            PathBuf::from("/a/e"),
        ];
        assert_eq!(zip_files(&paths), vec![PathBuf::from("/a/b.zip"), PathBuf::from("/a/c.ZIP")]);
    }
}
