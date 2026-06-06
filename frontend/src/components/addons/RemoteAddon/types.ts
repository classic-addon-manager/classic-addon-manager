import type { AddonManifest } from '@/lib/wails'

export interface RemoteAddonItemProps {
  manifest: AddonManifest
  installed: boolean
}
