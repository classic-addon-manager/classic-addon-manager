import type { Addon, Release } from '@/lib/wails'

/*
Decides whether a release should be offered as an update for an installed addon.
Releases without a tag are ignored, since there is no version to update to.
A different tag is always an update. The same tag counts as an update only when it
was re-published after the copy we installed. Addons saved without a publish time
can only be compared by tag. Mirrors hasUpdate in backend/addon/check_for_updates.go.
*/
export function hasAddonUpdate(
  addon: Addon,
  release: Release | null | undefined
): release is Release {
  if (!addon.isManaged || !release?.tag_name) return false
  if (release.tag_name !== addon.version) return true
  const installedAt = Date.parse(addon.updatedAt)
  if (!(installedAt > 0)) return false
  return Date.parse(release.published_at) > installedAt
}
