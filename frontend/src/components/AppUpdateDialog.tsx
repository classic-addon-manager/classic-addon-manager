import { useAtom, useAtomValue } from 'jotai'
import { ArrowRight, ArrowUpCircle, Download, LoaderCircle } from 'lucide-react'
import { useEffect, useState } from 'react'
import semver from 'semver'

import {
  updateAvailableAtom,
  updateDialogOpenAtom,
  updateInformationAtom,
  versionAtom,
} from '@/atoms/applicationAtoms'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ApplicationService } from '@/lib/wails'

export const AppUpdateDialog = () => {
  const currentVersion = useAtomValue(versionAtom)
  const [updateAvailable, setUpdateAvailable] = useAtom(updateAvailableAtom)
  const [updateInformation, setUpdateInformation] = useAtom(updateInformationAtom)
  const [open, setOpen] = useAtom(updateDialogOpenAtom)
  const [isUpdating, setIsUpdating] = useState(false)

  useEffect(() => {
    const checkForUpdates = async () => {
      try {
        const releaseInfo = await ApplicationService.GetLatestRelease()
        if (releaseInfo && semver.gt(releaseInfo.version, currentVersion)) {
          setUpdateInformation(releaseInfo)
          setUpdateAvailable(true)
          setOpen(true)
        }
      } catch (error) {
        console.error('Failed to get latest release:', error)
      }
    }

    checkForUpdates()
  }, [currentVersion, setUpdateAvailable, setUpdateInformation, setOpen])

  const handleUpdate = async () => {
    if (!updateInformation || isUpdating) return

    setIsUpdating(true)
    try {
      await ApplicationService.SelfUpdate(updateInformation.url)
    } catch (error) {
      console.error('Update failed:', error)
      setIsUpdating(false)
    }
  }

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen)
  }

  if (!updateAvailable || !updateInformation) {
    return null
  }

  const currentClean = currentVersion.replace(/^v/, '')
  const newClean = updateInformation.version.replace(/^v/, '')

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-emerald-500/20 ring-1 ring-emerald-500/30">
              <ArrowUpCircle className="w-5 h-5 text-emerald-400" />
            </div>
            <div>
              <DialogTitle>Update Available</DialogTitle>
              <DialogDescription>
                A new version of Classic Addon Manager is available.
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="flex items-center justify-center gap-2 rounded-lg border border-white/5 bg-zinc-900/50 px-4 py-3">
          <span className="rounded-md bg-white/5 px-2.5 py-1 text-sm font-mono text-white/50">
            {currentClean}
          </span>
          <ArrowRight className="w-4 h-4" />
          <span className="rounded-md bg-emerald-500/10 px-2.5 py-1 text-sm font-mono text-emerald-300 ring-1 ring-inset ring-emerald-500/20">
            {newClean}
          </span>
        </div>

        <DialogFooter className="gap-2">
          <Button variant="ghost" onClick={() => setOpen(false)} disabled={isUpdating}>
            Later
          </Button>
          <Button
            onClick={handleUpdate}
            disabled={isUpdating}
            className="bg-emerald-900 hover:bg-emerald-800 text-white shadow-md shadow-emerald-900/20"
          >
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
