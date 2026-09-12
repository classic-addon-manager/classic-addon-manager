import { BlocksIcon } from 'lucide-react'
import { useState } from 'react'

import type { OwnedAddon } from '@/components/developer/ownedParse'

export function AddonIcon({ addon }: { addon: OwnedAddon }) {
  const [failed, setFailed] = useState(false)
  return addon.branch && !failed ? (
    <img
      className="size-10 shrink-0 rounded-lg border object-cover"
      src={`https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`}
      alt=""
      onError={() => setFailed(true)}
    />
  ) : (
    <div className="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-card text-primary">
      <BlocksIcon className="size-5" />
    </div>
  )
}
