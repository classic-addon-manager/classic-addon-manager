#!/usr/bin/env bash
# Builds the release binary and packs it into an AppImage. Runs inside the Containerfile image (see `just appimage`).
# usage: build.sh <output-dir>
set -euo pipefail

out_dir=$(realpath -m "${1:?output directory required}")
src=$(pwd)
script_dir="$src/tools/appimage"
name=classic-addon-manager
arch=$(uname -m)

# Separate from the host's target/: it holds objects linked against the container's libraries.
export CARGO_TARGET_DIR="$src/target/appimage"
cargo build --release --locked
binary="$CARGO_TARGET_DIR/release/$name"

app_dir="$CARGO_TARGET_DIR/$name-$arch.AppDir"
rm -rf -- "$app_dir"
mkdir -p "$app_dir/usr/bin" "$app_dir/usr/lib" "$app_dir/usr/share/applications" "$app_dir/usr/share/icons/hicolor/256x256/apps"
install -m755 "$binary" "$app_dir/usr/bin/$name"
install -m755 "$script_dir/AppRun" "$app_dir/AppRun"
install -m644 "$script_dir/$name.desktop" "$app_dir/$name.desktop"
install -m644 "$script_dir/$name.desktop" "$app_dir/usr/share/applications/$name.desktop"
icon=crates/classic-addon-manager/assets/images/appicon.png
install -m644 "$icon" "$app_dir/$name.png"
install -m644 "$icon" "$app_dir/.DirIcon"
install -m644 "$icon" "$app_dir/usr/share/icons/hicolor/256x256/apps/$name.png"

# Bundle the linked non-glibc libraries (libxcb, libxkbcommon, ...). The glibc family and libgcc_s must come from the host.
ldd "$app_dir/usr/bin/$name" | awk '$2 == "=>" && $3 ~ /^\// { print $3 }' |
    grep -Ev '/(ld-linux[^/]*|lib(c|m|dl|rt|pthread|util|resolv|nsl|gcc_s))\.so[.0-9]*$' |
    while read -r lib; do
        install -m755 "$(readlink -f "$lib")" "$app_dir/usr/lib/$(basename "$lib")"
    done
for lib in "$app_dir"/usr/lib/*.so*; do
    patchelf --set-rpath '$ORIGIN' "$lib"
done
patchelf --set-rpath '$ORIGIN/../lib' "$app_dir/usr/bin/$name"

if ldd "$app_dir/usr/bin/$name" | grep -q 'not found'; then
    ldd "$app_dir/usr/bin/$name" >&2
    echo "unresolved libraries in the AppDir" >&2
    exit 1
fi

mkdir -p "$out_dir"
image="$out_dir/$name-$arch.AppImage"
ARCH="$arch" /opt/appimagetool/AppRun "$app_dir" "$image"
echo "built $image"
