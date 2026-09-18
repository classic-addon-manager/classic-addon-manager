/**
 * Resolves the icon URL for an addon.
 *
 * A catalog `icon` is already a full CDN URL and wins over any GitHub
 * fallback: the fallback exists only for owned addons that are not yet
 * synced to the catalog, where GitHub is the only place an icon can live.
 */
export function addonIconUrl(
  source: {
    icon?: string | null
    repo?: string
    branch?: string | null
  },
  opts?: { githubFallback?: boolean }
): string | null {
  if (source.icon) return source.icon
  if (opts?.githubFallback && source.repo && source.branch) {
    return `https://raw.githubusercontent.com/${source.repo}/${source.branch}/icon.png`
  }
  return null
}
