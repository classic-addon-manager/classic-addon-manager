import { Check, Download, LoaderCircle, ShieldQuestion } from 'lucide-react'

import { Badge } from '@/components/ui/badge'
import type { Addon } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'
import { useUpdateDialogStore } from '@/stores/updateDialogStore'

export const AddonStatus = ({ addon }: { addon: Addon }) => {
  return <RenderStatus addon={addon} />
}

const RenderStatus = ({ addon }: { addon: Addon }) => {
  const { isCheckingForUpdates, latestReleasesMap } = useAddonStore()
  const { open, setOpen } = useUpdateDialogStore()

  const latestRelease = latestReleasesMap.get(addon.name)

  if (addon.isManaged) {
    if (isCheckingForUpdates) {
      return (
        <div className="flex justify-center">
          <LoaderCircle className="mr-1 animate-spin text-primary" size={20} />
        </div>
      )
    } else if (latestRelease) {
      if (latestRelease?.published_at > addon.updatedAt) {
        // Update available
        return (
          <Badge
            variant="outline"
            className="py-1 cursor-pointer flex-shrink-0 flex-grow-0 text-primary border-primary/20 bg-primary/10 hover:bg-primary/20"
            onClick={e => {
              e.preventDefault()
              e.stopPropagation()
              if (!open) {
                setOpen(true)
              }
            }}
          >
            <Download size={14} className="mr-1" />
            Update ({latestRelease.tag_name})
          </Badge>
        )
      } else {
        // Up to date
        return (
          <Badge
            variant="outline"
            className="py-1 text-green-600 border-green-600/20 bg-green-500/10 hover:bg-green-500/20"
          >
            <Check size={14} className="mr-1" />
            Up to date
          </Badge>
        )
      }
    }
  } else {
    // Not managed addon
    return (
      <Badge className="py-1 cursor-pointer" variant="warning">
        <ShieldQuestion size={14} className="mr-2" />
        Not managed
      </Badge>
    )
  }
}
