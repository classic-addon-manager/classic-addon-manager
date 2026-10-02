#!/usr/bin/env bash
set -euo pipefail

script_dir=$(dirname "$(realpath "$0")")
binary=$(realpath "$1")
icon=$(realpath "$2")
desktop_file=$(realpath "$3")
mkdir -p "$4" "$5"
output_dir=$(realpath "$4")
build_dir=$(realpath "$5")

name=$(basename "$binary")
arch=$(uname -m)
app_dir="$build_dir/$name-$arch.AppDir"
image="$name-$arch.AppImage"
linuxdeploy="$build_dir/linuxdeploy-$arch.AppImage"

# Stage only the linked GTK4 stack. Wails' generator copies helpers from every
# installed WebKit stack and emits an image before we can relocate its runtime.
test ! -e "$app_dir" || rm -r -- "$app_dir"
mkdir -p "$app_dir/usr/bin" "$app_dir/usr/share/applications" "$app_dir/usr/share/pixmaps"
cp -a -- "$binary" "$app_dir/usr/bin/"
cp -a -- "$desktop_file" "$icon" "$app_dir/"
cp -a -- "$icon" "$app_dir/.DirIcon"
cp -a -- "$desktop_file" "$app_dir/usr/share/applications/"
cp -a -- "$icon" "$app_dir/usr/share/pixmaps/"

if ! test -x "$linuxdeploy"; then
  curl --fail --location --output "$linuxdeploy" \
    "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$arch.AppImage"
  chmod +x "$linuxdeploy"
fi

# Reuse the GTK plugin shipped with the project's pinned Wails dependency.
wails_dir=$(go list -m -f '{{.Dir}}' github.com/wailsapp/wails/v3)
cp -- "$wails_dir/internal/commands/linuxdeploy-plugin-gtk.sh" "$build_dir/"
chmod +x "$build_dir/linuxdeploy-plugin-gtk.sh"

python3 "$script_dir/runtime.py" stage "$app_dir" "$binary"
mapfile -t libraries < "$app_dir/runtime-libraries.txt"
rm -- "$app_dir/runtime-libraries.txt"
(
  cd "$build_dir"
  NO_STRIP=1 DEPLOY_GTK_VERSION=4 "$linuxdeploy" \
    --appimage-extract-and-run --appdir "$app_dir" "${libraries[@]}" --plugin gtk
)

# Relocate after dependency deployment. The final tool must not redeploy ELFs
# or overwrite their RPATHs and reintroduce host-library assumptions.
python3 "$script_dir/runtime.py" relocate "$app_dir" "$binary"
appimagetool="$build_dir/appimagetool-$arch.AppImage"
if ! test -x "$appimagetool"; then
  curl --fail --location --output "$appimagetool" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"
  chmod +x "$appimagetool"
fi
ARCH="$arch" "$appimagetool" --appimage-extract-and-run "$app_dir" "$build_dir/$image"
mv -f -- "$build_dir/$image" "$output_dir/$image"
