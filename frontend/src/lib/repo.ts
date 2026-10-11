import { useQuery } from '@tanstack/react-query'

import { addonReadmeQuery } from '@/lib/addonQueries'
import { fetchAddonCatalog } from '@/lib/catalog'
import type { AddonManifest } from '@/lib/wails'

export async function repoGetManifest(name: string): Promise<AddonManifest> {
  const manifests = await fetchAddonCatalog()
  const m = manifests.find(m => m.name === name)
  if (!m) {
    throw new Error(`Addon ${name} not found in repository manifests`)
  }
  return m
}

/**
 * Shared `['addon-readme', repo, branch]` query so the remote addon dialog and
 * the dashboard details pane share one README cache. Returns the README text,
 * `null` when the repo has no README, and throws on network failure so callers
 * can distinguish errors via `isError`.
 */
export function useAddonReadme(repo: string, branch: string | undefined, enabled = true) {
  return useQuery({
    ...addonReadmeQuery(repo, branch),
    enabled: enabled && !!repo,
  })
}
