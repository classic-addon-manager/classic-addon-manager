import { useQuery } from '@tanstack/react-query'

import { fetchAddonCatalog } from '@/lib/catalog'
import { safeCall } from '@/lib/utils'
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
  const normalizedBranch = branch || 'main'
  return useQuery({
    queryKey: ['addon-readme', repo, normalizedBranch],
    enabled: enabled && !!repo,
    retry: false,
    queryFn: async (): Promise<string | null> => {
      const [r, err] = await safeCall<Response>(
        fetch(`https://raw.githubusercontent.com/${repo}/refs/heads/${normalizedBranch}/README.md`)
      )
      if (err) {
        console.error('Error fetching README: ', err)
        throw err
      }
      if (!r || !r.ok) {
        return null
      }
      return r.text()
    },
  })
}
