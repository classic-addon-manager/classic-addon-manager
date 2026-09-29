#!/usr/bin/env bash
set -euo pipefail

binary=$1
icon=$2
desktop_file=$3
output_dir=$4
build_dir=$5

name=$(basename "$binary")
arch=$(uname -m)
app_dir="$build_dir/$name-$arch.AppDir"
image="$name-$arch.AppImage"
linuxdeploy="$build_dir/linuxdeploy-$arch.AppImage"
log=$(mktemp)
trap 'rm -f -- "$log"' EXIT

if wails3 generate appimage \
  -binary "$binary" \
  -icon "$icon" \
  -desktopfile "$desktop_file" \
  -outputdir "$output_dir" \
  -builddir "$build_dir" 2>&1 | tee "$log"; then
  exit 0
fi

# Wails currently copies WebKit helper processes from every installed GTK stack.
# On a GTK4 build, an unrelated WebKit 4.x helper can have missing dependencies.
binary_dependencies=$(ldd "$binary")
if ! grep -Fq 'Failed to deploy dependencies for existing files' "$log" ||
  [[ "$binary_dependencies" != *libwebkitgtk-6.0.so* ]] ||
  ! test -x "$linuxdeploy" ||
  ! test -f "$app_dir/AppRun" ||
  ! test -f "$build_dir/linuxdeploy-plugin-gtk.sh"; then
  exit 1
fi

mapfile -d '' old_webkit_dirs < <(find "$app_dir/usr/lib" -type d -name 'webkit2gtk-*' -prune -print0)
if ((${#old_webkit_dirs[@]} == 0)); then
  exit 1
fi

for old_webkit_dir in "${old_webkit_dirs[@]}"; do
  rm -r -- "$old_webkit_dir"
done

echo 'Retrying AppImage packaging with only the WebKitGTK 6.0 helpers.'
(
  cd "$build_dir"
  NO_STRIP=1 DEPLOY_GTK_VERSION=4 OUTPUT="$image" "$linuxdeploy" \
    --appimage-extract-and-run --appdir "$app_dir" --output appimage --plugin gtk
)
mv -f -- "$build_dir/$image" "$output_dir/$image"
