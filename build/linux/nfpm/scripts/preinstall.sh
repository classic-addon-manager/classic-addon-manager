#!/bin/sh

# Pre-install checks for Classic Addon Manager
echo "Checking dependencies for Classic Addon Manager..."

# Check WebKit2GTK 4.1 availability (apt will handle via depends: in the package,
# but verify it's actually installable)
if ! dpkg -s libwebkit2gtk-4.1-0 >/dev/null 2>&1; then
  echo "Note: libwebkit2gtk-4.1-0 will be installed as a dependency if needed."
fi

# Check GTK3
if ! dpkg -s libgtk-3-0 >/dev/null 2>&1; then
  echo "Note: libgtk-3-0 will be installed as a dependency if needed."
fi

exit 0