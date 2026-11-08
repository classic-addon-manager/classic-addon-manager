// Everything here is Windows-only, for other targets the script emits nothing.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Windows gives the main thread (where gpui lays out and paints) a 1 MiB stack by default, Linux 8 MiB. Reserve
    // 8 MiB on Windows too: rendering recurses once per nesting level of a Markdown document, and even the depth that
    // `markdown_html::MAX_DEPTH` allows overflows 1 MiB in debug builds. Only address space is reserved up front.
    match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => println!("cargo:rustc-link-arg-bins=/STACK:8388608"),
        Ok("gnu") => println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608"),
        _ => {}
    }

    // Application icon (resources/windows/app.rc). `manifest_required` turns a missing resource compiler into a build
    // error, otherwise the exe, title bar and taskbar would silently get the generic Windows icon.
    println!("cargo:rerun-if-changed=resources/windows");
    embed_resource::compile("resources/windows/app.rc", embed_resource::ParamsIncludeDirs(["resources/windows"]))
        .manifest_required()
        .unwrap();
}
