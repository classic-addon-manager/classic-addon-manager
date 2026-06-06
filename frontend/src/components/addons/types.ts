import type { AddonManifest } from '@/lib/wails'

export type AddonViewMode = 'list' | 'grid'

export type AddonListItem = {
  manifest: AddonManifest
  isInstalled: boolean
}
