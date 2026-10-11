import { queryOptions } from '@tanstack/react-query'

import { queryClient } from '@/lib/queryClient'
import { safeCall } from '@/lib/utils'
import type { AddonManifest } from '@/lib/wails'
import { RemoteAddonService } from '@/lib/wails'

const FIVE_MINUTES = 5 * 60 * 1000

/**
 * Query definitions for the remote addon dialog. They live apart from the hooks so the addon
 * list can prefetch them on hover, they only throw, leaving error toasts to the dialog.
 */
export const addonReleaseQuery = (name: string) =>
  queryOptions({
    queryKey: ['addon-release', name],
    retry: false,
    staleTime: FIVE_MINUTES,
    queryFn: async () => {
      const [release, err] = await safeCall(RemoteAddonService.GetLatestRelease(name))
      if (err) {
        console.error('Fetch release error: ', err)
        throw err
      }
      return release
    },
  })

export const addonReadmeQuery = (repo: string, branch: string | undefined) => {
  const normalizedBranch = branch || 'main'
  return queryOptions({
    queryKey: ['addon-readme', repo, normalizedBranch],
    retry: false,
    staleTime: FIVE_MINUTES,
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

export const addonDependenciesQuery = (manifest: AddonManifest) =>
  queryOptions({
    queryKey: ['addon-deps', manifest.name],
    retry: false,
    staleTime: FIVE_MINUTES,
    queryFn: async () => {
      const [result, err] = await safeCall(RemoteAddonService.ResolveDependencies(manifest))
      if (err || !result) throw err ?? new Error('Failed to resolve dependencies')
      return result
    },
  })

/** Failures are ignored here, the dialog reports its own errors when it opens. */
const ignoreError = () => undefined

/**
 * Warms the cache for everything the addon dialog shows, so it opens with data already there.
 * `queryClient.query` only fetches when the cached data is stale.
 */
export const prefetchAddonDetails = (manifest: AddonManifest) => {
  if (manifest.repo) {
    queryClient.query(addonReadmeQuery(manifest.repo, manifest.branch)).catch(ignoreError)
  }
  queryClient.query(addonReleaseQuery(manifest.name)).catch(ignoreError)
  if (manifest.dependencies.length > 0) {
    queryClient.query(addonDependenciesQuery(manifest)).catch(ignoreError)
  }
}
