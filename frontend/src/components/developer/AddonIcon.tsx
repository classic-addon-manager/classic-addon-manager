import { BlocksIcon } from 'lucide-react'
import { useState } from 'react'

import type { OwnedAddon } from '@/components/developer/ownedParse'
import { addonIconUrl } from '@/lib/icon'

export function AddonIcon({ addon }: { addon: OwnedAddon }) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null)
  const iconUrl = addonIconUrl(addon, { githubFallback: true })
  const hasIcon = iconUrl !== null && failedUrl !== iconUrl

  return hasIcon ? (
    <img
      className="size-10 shrink-0 rounded-lg border object-cover"
      src={iconUrl}
      alt=""
      onError={() => setFailedUrl(iconUrl)}
    />
  ) : (
    <div className="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-card text-primary">
      <BlocksIcon className="size-5" />
    </div>
  )
}
