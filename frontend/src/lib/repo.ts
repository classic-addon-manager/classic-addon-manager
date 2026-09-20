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
