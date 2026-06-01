import { Browser } from '@wailsio/runtime'
import { useAtomValue } from 'jotai'
import { Heart } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

import { updateCheckCompleteAtom, updateDialogOpenAtom } from '@/atoms/applicationAtoms'
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

export const SupportProjectDialog = () => {
  const updateCheckComplete = useAtomValue(updateCheckCompleteAtom)
  const updateDialogOpen = useAtomValue(updateDialogOpenAtom)
  const [open, setOpen] = useState(false)
  const recordedRef = useRef(false)
  const evaluatedRef = useRef(false)

  const KOFI_URL = 'https://ko-fi.com/gaijindev'

  const recordShown = async () => {
    if (recordedRef.current) return
    recordedRef.current = true
    try {
      await ApplicationService.RecordKofiModalShown()
    } catch (error) {
      console.error('Failed to record kofi modal shown:', error)
    }
  }

  useEffect(() => {
    if (!updateCheckComplete || updateDialogOpen || evaluatedRef.current) {
      return
    }
    evaluatedRef.current = true

    const evaluate = async () => {
      try {
        const shouldShow = await ApplicationService.ShouldShowKofiModal()
        if (shouldShow) {
          setOpen(true)
        } else {
          await ApplicationService.EnsureAppStateInitialized()
        }
      } catch (error) {
        console.error('Failed to check kofi modal eligibility:', error)
      }
    }

    void evaluate()
  }, [updateCheckComplete, updateDialogOpen])

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen)
    if (!nextOpen) {
      void recordShown()
    }
  }

  const handleSupport = () => {
    Browser.OpenURL(KOFI_URL)
    setOpen(false)
    void recordShown()
  }

  if (!open) {
    return null
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <div className="flex items-start gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-rose-500/20 ring-1 ring-rose-500/30">
              <Heart className="size-5 shrink-0 text-rose-400" strokeWidth={2} />
            </div>
            <div>
              <DialogTitle>Enjoying Classic Addon Manager?</DialogTitle>
              <DialogDescription>Free to use, supported by players like you</DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="flex flex-col items-center gap-4 rounded-lg border border-white/5 bg-zinc-900/50 px-4 py-5">
          <p className="text-center text-sm text-muted-foreground">
            Classic Addon Manager is an independent project developed and maintained by Sami for
            ArcheAge Classic.
          </p>

          <p className="text-center text-sm text-muted-foreground">
            Tips on Ko-fi help fund infrastructure and continued development.{' '}
          </p>

          <p className="text-center text-sm text-muted-foreground">
            Thanks for supporting the project!
          </p>
        </div>

        <DialogFooter className="gap-2 sm:justify-end">
          <Button variant="ghost" onClick={() => handleOpenChange(false)}>
            Maybe later
          </Button>
          <Button
            onClick={handleSupport}
            className="bg-rose-900 hover:bg-rose-800 text-white shadow-md shadow-rose-900/20"
          >
            <Heart className="size-4 shrink-0" strokeWidth={2} />
            Support on Ko-fi
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
