import {
  AlertTriangleIcon,
  ArrowRight,
  ArrowUpCircle,
  CalendarDays,
  Download,
  LoaderCircle,
} from 'lucide-react'
import { useState } from 'react'

import { Readme } from '@/components/shared/Readme'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { toast } from '@/components/ui/toast'
import { repoGetManifest } from '@/lib/repo.ts'
import { formatToLocalTime, safeCall } from '@/lib/utils'
import type { Addon, Release } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'
import { useUpdateDialogStore } from '@/stores/updateDialogStore'

interface Props {
  addon: Addon
  release: Release
}

export const LocalAddonUpdateDialog = ({ addon, release }: Props) => {
  const [isUpdating, setIsUpdating] = useState(false)
  const { open, setOpen } = useUpdateDialogStore()
  const { update } = useAddonStore()

  const changelog = release?.body?.trim() ?? ''
  const hasChangelog = changelog.length > 0

  const handleUpdateClick = async () => {
    if (isUpdating) return

    setIsUpdating(true)

    const updateOperation = async () => {
      const manifest = await repoGetManifest(addon.name)
      return await update(manifest, release.tag_name)
    }

    const [didInstall, err] = await safeCall<boolean>(updateOperation())

    if (err) {
      if (err.message.includes('not found')) {
        toast({
          title: 'Error',
          description: 'No release found for this addon',
          icon: AlertTriangleIcon,
        })
      } else {
        toast({
          title: 'Error',
          description: `Failed to update addon: ${err.message.substring(0, 100)}`,
        })
      }
      setIsUpdating(false)
      return
    }

    if (didInstall) {
      toast({
        title: 'Addon updated',
        description: `${addon.alias} was updated to ${release.tag_name}`,
        icon: ArrowUpCircle,
      })
      setOpen(false)
    }
    setIsUpdating(false)
  }

  return (
    <Dialog onOpenChange={setOpen} open={open}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-amber-500/15 ring-1 ring-amber-500/25">
              <ArrowUpCircle className="w-5 h-5 text-amber-500" />
            </div>
            <div>
              <DialogTitle>Update available</DialogTitle>
              <DialogDescription>{addon.alias} has a new version.</DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="flex items-center justify-center gap-2 rounded-lg border border-border/60 bg-muted/40 px-4 py-3">
          <span className="rounded-md bg-muted px-2.5 py-1 text-sm font-mono text-muted-foreground">
            {addon.version}
          </span>
          <ArrowRight className="w-4 h-4 text-muted-foreground" />
          <span className="rounded-md bg-amber-500/10 px-2.5 py-1 text-sm font-mono text-amber-600 dark:text-amber-400 ring-1 ring-inset ring-amber-500/20">
            {release.tag_name}
          </span>
        </div>

        <div className="space-y-2">
          {release?.published_at && (
            <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
              <CalendarDays className="w-3.5 h-3.5" />
              <span>Released {formatToLocalTime(release.published_at)}</span>
            </div>
          )}

          {hasChangelog ? (
            <div className="max-h-48 overflow-y-auto rounded-lg border border-border/60 bg-card p-3 text-sm">
              <Readme readme={changelog} />
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No release notes provided.</p>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="ghost" onClick={() => setOpen(false)} disabled={isUpdating}>
            Later
          </Button>
          <Button onClick={handleUpdateClick} disabled={isUpdating}>
            {isUpdating ? (
              <>
                <LoaderCircle className="w-4 h-4 mr-2 animate-spin" />
                Updating…
              </>
            ) : (
              <>
                <Download className="w-4 h-4" />
                Update Now
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
