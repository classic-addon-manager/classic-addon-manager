import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  // DialogFooter,
} from '@/components/ui/dialog'
import type { Addon } from '@/lib/wails'
import { PackageIcon, Tag, User } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Suspense, useEffect, useState } from 'react'
import { RemoteAddonReadme } from '@/components/shared/RemoteAddonReadme'
import { AddonRepositoryMatch } from '@/components/dashboard/AddonRepositoryMatch'

interface LocalAddonDialogProps {
  addon: Addon
  onOpenChange: (open: boolean) => void
  open: boolean
}

export const LocalAddonDialog = ({ addon, onOpenChange, open }: LocalAddonDialogProps) => {
  const [readme, setReadme] = useState<string>('loading')
  const [initiallyManaged, setInitiallyManaged] = useState(false)
  const [hasBeenOpened, setHasBeenOpened] = useState(false)

  // Set initial managed state when dialog opens
  useEffect(() => {
    if (open && !hasBeenOpened) {
      // Mark that we've opened the dialog and captured the initial state
      setHasBeenOpened(true)
      setInitiallyManaged(addon.isManaged)
    } else if (!open) {
      // Reset when dialog closes
      setHasBeenOpened(false)
    }
  }, [open, addon.isManaged, hasBeenOpened])

  // Close dialog when addon becomes managed (if it wasn't initially managed)
  useEffect(() => {
    // Only close if:
    // 1. Dialog is open
    // 2. We've already captured the initial state (hasBeenOpened)
    // 3. Addon is now managed
    // 4. It wasn't managed initially
    if (open && hasBeenOpened && addon.isManaged && !initiallyManaged) {
      onOpenChange(false)
    }
  }, [addon.isManaged, onOpenChange, open, initiallyManaged, hasBeenOpened])

  // Fetch readme when dialog opens
  useEffect(() => {
    if (!open) {
      return
    }
    getReadme()
  }, [open])

  const getReadme = async () => {
    const fallbackReadme = addon.description || 'No description provided'

    if (!addon.repo) {
      setReadme(fallbackReadme)
      return
    }

    const branch = addon.branch || 'main'

    const response = await fetch(
      `https://raw.githubusercontent.com/${addon.repo}/${branch}/README.md`
    )
    if (!response.ok) {
      setReadme(fallbackReadme)
      return
    }

    const text = await response.text()
    setReadme(text)
  }

  const UnmanagedAddonNotice = () => {
    return (
      <div className="border rounded-lg p-4 bg-card mt-6">
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-muted-foreground">
            <PackageIcon className="w-4 h-4" />
            <p className="text-sm font-medium">
              This addon is not managed by Classic Addon Manager
            </p>
          </div>

          <Suspense
            fallback={
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <div className="w-3 h-3 rounded-full border-2 border-t-transparent border-current animate-spin"></div>
                <span>Checking repository...</span>
              </div>
            }
          >
            <AddonRepositoryMatch name={addon.name} />
          </Suspense>
        </div>
      </div>
    )
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-w-0 min-w-[650px] md:max-w-[70svw] min-h-[500px] max-h-[90svh] lg:max-w-[850px] flex flex-col p-0">
        <DialogHeader className="p-6 pb-4 shrink-0 border-b">
          <DialogTitle className="text-2xl font-semibold mb-1">{addon.alias}</DialogTitle>

          {/* Display author and version information for managed addons */}
          {addon.isManaged && (
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground">
              <span className="inline-flex items-center gap-1">
                <User className="w-3.5 h-3.5" />
                <span className="font-normal text-foreground/90">{addon.author}</span>
              </span>

              {addon.version ? (
                <Badge variant="secondary" className="inline-flex items-center gap-1">
                  <Tag className="w-3 h-3" />
                  {addon.version}
                </Badge>
              ) : (
                <Badge variant="outline">No Version</Badge>
              )}
            </div>
          )}

          {addon.isManaged && addon.description && (
            <DialogDescription className="text-sm text-muted-foreground">
              {addon.description}
            </DialogDescription>
          )}
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-6 pb-6 pt-2">
          {addon.isManaged && readme === 'loading' ? (
            <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
              <PackageIcon className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">Loading description...</p>
            </div>
          ) : (
            addon.isManaged && <RemoteAddonReadme readme={readme} />
          )}

          {/* Show notice if addon is not managed and is available in repository */}
          {!addon.isManaged && UnmanagedAddonNotice()}
        </div>
      </DialogContent>
    </Dialog>
  )
}
