import type { OwnedAddon } from '@/components/developer/ownedParse'
import { AddonIconImage } from '@/components/shared/AddonIconImage'
import { addonIconUrl } from '@/lib/icon'

export function AddonIcon({ addon }: { addon: OwnedAddon }) {
  return (
    <AddonIconImage
      src={addonIconUrl(addon, { githubFallback: true })}
      alt=""
      imageClassName="size-10 shrink-0 rounded-lg border object-cover"
      fallbackClassName="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-card text-primary"
      fallbackIconClassName="size-5"
    />
  )
}
