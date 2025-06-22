import { Addon } from '@/lib/wails'
import { Download, LoaderCircle, Check, ShieldQuestion } from 'lucide-react'
import { useAddonStore } from '@/stores/addonStore'
import { Badge } from '@/components/ui/badge'

export const AddonStatus = ({ addon }: { addon: Addon }) => {
  // const isContextMenuOpen = false // TODO: This should be an atom I think.
  // let openDialog = false // TODO: Figure out what to do about this

  return <RenderStatus addon={addon} />
}

const RenderStatus = ({ addon }: { addon: Addon }) => {
  const { isCheckingForUpdates, latestReleasesMap } = useAddonStore()

  const latestRelease = latestReleasesMap.get(addon.name)

  if (addon.isManaged) {
    if (isCheckingForUpdates) {
      return (
        <div className="flex justify-center">
          <LoaderCircle className="mr-1 animate-spin" size={20} />
        </div>
      )
    } else if (latestRelease) {
      if (latestRelease?.published_at > addon.updatedAt) {
        // Update available
        return (
          <Badge
            className="py-1 cursor-pointer flex-shrink-0 flex-grow-0"
            onClick={e => {
              e.preventDefault()
              e.stopPropagation()
              // openUpdateDialog = true
            }}
          >
            <div style={{ width: 14, height: 14 }}>
              <Download size={14} className="mr-1" />
            </div>
            Update ({latestRelease.tag_name})
          </Badge>
        )
      } else {
        // Up to date
        return (
          <div className="flex justify-center">
            <Check size={20} className="text-green-600 mr-2" />
            Up to date
          </div>
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
