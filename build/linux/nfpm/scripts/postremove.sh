#!/bin/sh

# Remove desktop file if it exists
if [ -f "/usr/share/applications/classic-addon-manager.desktop" ]; then
  echo "Removing desktop file..."
  rm -f /usr/share/applications/classic-addon-manager.desktop
fi

# Update desktop database to reflect the removal
if command -v update-desktop-database >/dev/null 2>&1; then
  echo "Updating desktop database..."
  update-desktop-database -q /usr/share/applications
fi

# Update MIME database
if command -v update-mime-database >/dev/null 2>&1; then
  echo "Updating MIME database..."
  update-mime-database -n /usr/share/mime
fi

exit 0