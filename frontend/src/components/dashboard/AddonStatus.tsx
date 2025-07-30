import { useAtom, type WritableAtom } from 'jotai'
import { Check, Download, LoaderCircle, ShieldQuestion } from 'lucide-react'
import { useContext } from 'react'

import { UpdateDialogAtomContext } from '@/components/dashboard/contexts'
import { Badge } from '@/components/ui/badge'
import type { Addon } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

export const AddonStatus = ({ addon }: { addon: Addon }) => {
  const updateDialogAtom = useContext(UpdateDialogAtomContext)

  return <RenderStatus addon={addon} updateDialogAtom={updateDialogAtom} />
}

const RenderStatus = ({
  addon,
  updateDialogAtom,
}: {
  addon: Addon
  updateDialogAtom: WritableAtom<boolean, any, any> | null
}) => {
  const { isCheckingForUpdates, latestReleasesMap } = useAddonStore()
  const setOpen = updateDialogAtom ? useAtom(updateDialogAtom)[1] : null

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
              if (setOpen) {
                setOpen(true)
              }
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
