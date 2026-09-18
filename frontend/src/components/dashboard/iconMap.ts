import { atom } from 'jotai'

import { addonIconUrl } from '@/lib/icon'
import { RemoteAddonService } from '@/lib/wails'

/**
 * Icon URL per addon name, from the catalog manifests. Loaded once by the
 * dashboard controller and shared by the list and the details pane, so rows
 * do not each trigger a `GET /addons` fetch. Addons missing from the catalog
 * (or without a synced icon) have no entry and show the placeholder.
 */
export const catalogIconMapAtom = atom<Map<string, string>>(new Map())

export const loadCatalogIconMapAtom = atom(null, async (_get, set) => {
  try {
    const manifests = await RemoteAddonService.GetAddonManifest()
    const iconByName = new Map<string, string>()
    for (const manifest of manifests) {
      const iconUrl = addonIconUrl(manifest)
      if (iconUrl) iconByName.set(manifest.name, iconUrl)
    }
    set(catalogIconMapAtom, iconByName)
  } catch {
    // Icons are cosmetic: an unreachable catalog leaves the map empty and
    // every row falls back to the placeholder.
    set(catalogIconMapAtom, new Map())
  }
})
