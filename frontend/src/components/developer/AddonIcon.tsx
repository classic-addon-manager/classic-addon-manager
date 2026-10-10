import type { OwnedAddon } from '@/components/developer/ownedParse'
import { AddonIcon as SharedAddonIcon } from '@/components/shared/AddonIcon'
import { addonIconUrl } from '@/lib/icon'

export function AddonIcon({ addon }: { addon: OwnedAddon }) {
  return <SharedAddonIcon name={addon.alias} src={addonIconUrl(addon, { githubFallback: true })} />
}
