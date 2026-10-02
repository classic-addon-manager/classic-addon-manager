#!/usr/bin/env python3
"""Stage and relocate the linked production WebKitGTK runtime, never host /usr."""

import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys


# Only runtime constants: never rewrite /usr generally (sandbox LIBDIR, debug
# strings and system graphics-driver paths must retain their original meaning).
RUNTIME_PATH = re.compile(
    rb"/usr/(?:lib(?:64|/[\w-]+)?|libexec|share)/webkitgtk-6\.0(?:/[\w./-]*)?\x00"
    rb"|/usr/bin/(?:bwrap|xdg-dbus-proxy)\x00"
)


def output(*args):
    return subprocess.check_output(args, text=True).strip()


def linked_webkit(binary):
    match = re.search(r"libwebkitgtk-6\.0\.so\S* => (\S+)", output("ldd", str(binary)))
    if not match or not Path(match[1]).is_file():
        raise RuntimeError("AppImage packaging requires the linked WebKitGTK 6.0 library")
    return Path(match[1]).resolve()


def runtime_paths(library):
    return sorted({m.group()[:-1].decode() for m in RUNTIME_PATH.finditer(library.read_bytes())})


def copy_path(source, appdir):
    source = Path(source)
    if not source.is_absolute():
        raise RuntimeError(f"Runtime installation path must be absolute: {source}")
    target = appdir / str(source).lstrip("/")
    target.parent.mkdir(parents=True, exist_ok=True)
    if source.is_dir():
        shutil.copytree(source, target, symlinks=False, dirs_exist_ok=True)
    else:
        shutil.copy2(source, target, follow_symlinks=True)
    return target


def stage(appdir, binary):
    library = linked_webkit(binary)
    paths = runtime_paths(library)
    helper_dirs = [Path(p) for p in paths
                   if Path(p).name == "webkitgtk-6.0" and (Path(p) / "WebKitWebProcess").is_file()]
    if len(helper_dirs) != 1:
        raise RuntimeError(f"Expected one compiled WebKit helper directory, found {helper_dirs}")
    helpers = helper_dirs[0]
    for name in ("WebKitNetworkProcess", "WebKitWebProcess"):
        if not os.access(helpers / name, os.X_OK):
            raise RuntimeError(f"Missing executable WebKit helper: {helpers / name}")
    if b"WebKitGPUProcess\x00" in library.read_bytes() and not os.access(helpers / "WebKitGPUProcess", os.X_OK):
        raise RuntimeError("This WebKit build requires WebKitGPUProcess, but it is not installed")
    copy_path(helpers, appdir)  # Includes GPUProcess when built.
    bundles = [Path(p) for p in paths if "injected-bundle" in p]
    if not bundles:
        raise RuntimeError("WebKit's compiled injected-bundle path was not found")
    for bundle in bundles:
        if not bundle.exists():
            raise RuntimeError(f"Missing WebKit injected bundle: {bundle}")
        if not bundle.is_relative_to(helpers):
            copy_path(bundle, appdir)  # Fedora separates libexec helpers from lib64 bundles.
    for path in paths:
        if "/share/" in path and Path(path).exists():
            copy_path(path, appdir)
    resources = Path("/usr/share/webkitgtk-6.0")
    if resources.is_dir():
        copy_path(resources, appdir)

    for tool in ("bwrap", "xdg-dbus-proxy"):
        source = Path(f"/usr/bin/{tool}")
        if not os.access(source, os.X_OK) or str(source) not in paths:
            raise RuntimeError(f"Unsupported or missing WebKit sandbox executable: {tool}")
        copy_path(source, appdir)

    plugins = Path(output("pkg-config", "--variable=pluginsdir", "gstreamer-1.0"))
    scanner = Path(output("pkg-config", "--variable=pluginscannerdir", "gstreamer-1.0")) / "gst-plugin-scanner"
    if not plugins.is_dir() or not scanner.is_file():
        raise RuntimeError("Install GStreamer plugins and gst-plugin-scanner before packaging")
    copy_path(plugins, appdir)
    copy_path(scanner, appdir)
    # GIO loads TLS/proxy modules dynamically; ldd cannot discover them.
    gio_modules = Path(output("pkg-config", "--variable=giomoduledir", "gio-2.0"))
    if not gio_modules.is_dir():
        raise RuntimeError("Install the GIO TLS modules before packaging")
    copy_path(gio_modules, appdir)
    copy_path(Path("/etc/fonts"), appdir)

    # Use original module paths so linuxdeploy also finds their package licenses.
    libraries = [p.resolve() for root in (plugins, gio_modules) for p in root.rglob("*.so")]
    # linuxdeploy's default blacklist assumes a full build-distro desktop.
    # Explicitly deploy the dependency closure except the host ABI/driver layer.
    platform_libraries = {
        "libc.so.6", "libm.so.6", "libdl.so.2", "libpthread.so.0", "librt.so.1",
        "libresolv.so.2", "libutil.so.1", "libanl.so.1", "libstdc++.so.6", "libgcc_s.so.1",
        "libEGL.so.1", "libGL.so.1", "libGLX.so.0", "libGLdispatch.so.0",
        "libGLESv2.so.2", "libGLESv1_CM.so.1", "libglapi.so.0", "libgbm.so.1", "libdrm.so.2",
        "libwayland-client.so.0", "libwayland-server.so.0",
    }
    roots = [binary, *libraries, appdir / "usr/bin/bwrap", appdir / "usr/bin/xdg-dbus-proxy",
             appdir / str(scanner).lstrip("/")]
    dependencies = re.findall(r"=> (/\S+)", output("ldd", *(str(p) for p in roots)))
    for path in dependencies:
        if Path(path).name not in platform_libraries and not Path(path).name.startswith(("ld-linux", "libdrm_")):
            libraries.append(Path(path).resolve())
    (appdir / "runtime-libraries.txt").write_text(
        "".join(f"--library={p}\n" for p in sorted(set(libraries)))
    )
    gst_version = output("pkg-config", "--modversion", "gstreamer-1.0")
    config = (
        f'export GST_PLUGIN_SYSTEM_PATH_1_0="$APPDIR/{str(plugins).lstrip("/")}"\n'
        'export GST_PLUGIN_PATH_1_0=\n'
        'export GST_PLUGIN_PATH=\n'
        f'export GST_PLUGIN_SCANNER="$APPDIR/{str(scanner).lstrip("/")}"\n'
        'export GST_PLUGIN_SCANNER_1_0="$GST_PLUGIN_SCANNER"\n'
        f'export GST_REGISTRY="${{XDG_CACHE_HOME:-$HOME/.cache}}/classic-addon-manager/gstreamer/registry-{gst_version}-{os.uname().machine}.bin"\n'
        'mkdir -p "$(dirname "$GST_REGISTRY")"\n'
        'export GST_REGISTRY_1_0="$GST_REGISTRY"\n'
        f'export GIO_EXTRA_MODULES="$APPDIR/{str(gio_modules).lstrip("/")}"\n'
        'export FONTCONFIG_PATH="$APPDIR/etc/fonts"\n'
        'export FONTCONFIG_FILE="$FONTCONFIG_PATH/fonts.conf"\n'
    )
    (appdir / "runtime-env.sh").write_text(config)


def relocate(appdir, binary):
    # Explicit real paths collect licenses. Provide loader SONAME aliases,
    # including libraries linuxdeploy skips during automatic deployment, and
    # replace duplicate SONAME files with normal links to versioned files.
    for candidate in (appdir / "usr/lib").glob("*.so.*"):
        if candidate.is_symlink():
            continue
        soname = output("patchelf", "--print-soname", str(candidate))
        canonical = candidate.with_name(soname) if soname else candidate
        if canonical != candidate and not canonical.is_symlink():
            canonical.unlink(missing_ok=True)
            canonical.symlink_to(candidate.name)

    library = linked_webkit(binary)
    paths = runtime_paths(library)
    bundled = appdir / "usr/lib" / output("patchelf", "--print-soname", str(library))
    data = bundled.read_bytes()
    for path in paths:
        old = path.encode() + b"\0"
        new = b"././" + old[4:]
        if old not in data:
            raise RuntimeError(f"Bundled WebKit runtime constant missing: {path}")
        # Same length: preserve ELF layout, references and any suffix sharing.
        data = data.replace(old, new)
    bundled.write_bytes(data)

    # WTF's container capability probe lives in JavaScriptCore, not libwebkit.
    # Helpers/injected bundles may also contain their own compiled constants.
    candidates = set((appdir / "usr/lib").glob("libjavascriptcoregtk-6.0.so*"))
    for path in paths:
        root = appdir / path.lstrip("/")
        if root.is_dir():
            candidates.update(p for p in root.rglob("*") if p.is_file())
    for candidate in sorted({p.resolve() for p in candidates}):
        data = candidate.read_bytes()
        if data.startswith(b"\x7fELF") and RUNTIME_PATH.search(data):
            candidate.write_bytes(RUNTIME_PATH.sub(lambda m: b"././" + m.group()[4:], data))

    # Mesa loads these too; an older bundled Wayland core can hide newer
    # driver-required symbols (e.g. wl_display_create_queue_with_name).
    for component in ("client", "server"):
        for library in (appdir / "usr/lib").glob(f"libwayland-{component}.so*"):
            library.unlink()

    # Keep the sandbox enabled. Its production build doesn't bind AppImage
    # prefixes; expose this read-only runtime and preserve its relative cwd.
    bwrap = appdir / "usr/bin/bwrap"
    real_bwrap = appdir / "usr/bin/bwrap.real"
    bwrap.rename(real_bwrap)
    bwrap.write_text(Path(__file__).with_name("bwrap").read_text())
    bwrap.chmod(0o755)
    # Discard linuxdeploy's initial launcher backup. Its output pass must wrap
    # our launcher, not resurrect the original symlink straight to the binary.
    (appdir / "AppRun.wrapped").unlink(missing_ok=True)
    (appdir / "AppRun").unlink(missing_ok=True)
    template = Path(__file__).with_name("AppRun").read_text()
    (appdir / "AppRun").write_text(template.replace("@BINARY@", shlex.quote(binary.name)))
    (appdir / "AppRun").chmod(0o755)


if __name__ == "__main__":
    mode, directory, executable = sys.argv[1:]
    {"stage": stage, "relocate": relocate}[mode](Path(directory).resolve(), Path(executable).resolve())
