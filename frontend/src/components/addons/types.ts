import type { AddonManifest } from '@/lib/wails'

export type AddonListItem = {
  manifest: AddonManifest
  isInstalled: boolean
}
