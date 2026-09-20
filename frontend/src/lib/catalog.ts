import { useQuery } from '@tanstack/react-query'

import { queryClient } from '@/lib/queryClient'
import { type AddonManifest, RemoteAddonService } from '@/lib/wails'

/**
 * Shared options for the whole-catalog fetch. One `['addon-catalog']` query
 * gives every consumer dedup, shared staleness, and a single invalidation
 * point instead of each site fetching `GetAddonManifest` on its own.
 */
export const addonCatalogQuery = {
  queryKey: ['addon-catalog'],
  staleTime: 5 * 60 * 1000,
  queryFn: () => RemoteAddonService.GetAddonManifest(),
} as const

export function useAddonCatalog() {
  return useQuery(addonCatalogQuery)
}

/**
 * Catalog read for non-hook callers (jotai atoms, event handlers). Resolves
 * from cache while fresh and dedups concurrent fetches.
 */
export function fetchAddonCatalog(): Promise<AddonManifest[]> {
  return queryClient.query(addonCatalogQuery)
}
