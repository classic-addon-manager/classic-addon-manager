import { useAddonCatalog } from '@/lib/catalog'
import type { Addon, AddonManifest } from '@/lib/wails'

export type Deprecation = {
  reason: string | null
  /** Catalog name of the addon the developer recommends instead, when they named one. */
  replacement: string | null
}

type DeprecationFields = Pick<AddonManifest, 'deprecated' | 'deprecation_reason' | 'replacement'>

export function deprecationOf(manifest: DeprecationFields | null | undefined): Deprecation | null {
  if (!manifest?.deprecated) return null
  return {
    reason: manifest.deprecation_reason || null,
    replacement: manifest.replacement || null,
  }
}

/** One-line explanation shown in tooltips and banners. */
export function deprecationMessage(
  deprecation: Deprecation,
  replacement: Pick<AddonManifest, 'alias'> | null = null
): string {
  const reason = deprecation.reason ? ` ${deprecation.reason}` : ''
  const instead = replacement ? ` Its developer recommends ${replacement.alias} instead.` : ''
  return `This addon is deprecated and should be uninstalled.${reason}${instead}`
}

/**
 * Deprecation of an installed addon, read from the catalog by name. Manual
 * addons are not linked to a catalog entry, so they are never flagged, and an
 * unreachable catalog flags nothing.
 */
export function useAddonDeprecation(addon: Pick<Addon, 'name' | 'isManaged'>): Deprecation | null {
  const { data: catalog } = useAddonCatalog()
  if (!addon.isManaged) return null
  return deprecationOf(catalog?.find(manifest => manifest.name === addon.name))
}

/** Catalog entry for a replacement name, so it can be shown by alias and opened. */
export function useReplacementManifest(deprecation: Deprecation | null): AddonManifest | null {
  const { data: catalog } = useAddonCatalog()
  const name = deprecation?.replacement
  if (!name) return null
  const manifest = catalog?.find(entry => entry.name === name)
  // A replacement that is itself deprecated is no longer a recommendation.
  return manifest && !manifest.deprecated ? manifest : null
}
