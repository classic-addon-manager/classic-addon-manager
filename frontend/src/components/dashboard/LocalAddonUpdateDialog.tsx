import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Addon, Release } from '@/lib/wails'
import { useContext, useEffect, useState } from 'react'
import { UpdateDialogAtomContext } from '@/components/dashboard/LocalAddon'
import { useAtom } from 'jotai'
import { CalendarDays, Package, Tag, User } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { formatToLocalTime } from '@/lib/utils'
import Markdown from 'react-markdown'

interface Props {
  addon: Addon
  release: Release
}

export const LocalAddonUpdateDialog = ({ addon, release }: Props) => {
  const atomContext = useContext(UpdateDialogAtomContext)
  if (!atomContext) {
    throw new Error('LocalAddonUpdateDialog must be used within UpdateDialogAtomContext')
  }

  const [open, setOpen] = useAtom(atomContext)

  const [changelog, setChangelog] = useState<string>('')

  useEffect(() => {
    if (open && release?.body) {
      setChangelog(release.body)
    } else {
      setChangelog('No change log was provided')
    }
  }, [open])

  const ReleaseInformation = () => {
    if (!release) {
      return <Badge variant="outline">No Release</Badge>
    }
    return (
      <span className="inline-flex items-center gap-x-1.5">
        <Badge variant="secondary" className="inline-flex items-center gap-1">
          <Tag className="w-3 h-3" />
          {release.tag_name}
        </Badge>
      </span>
    )
  }

  return (
    <Dialog onOpenChange={setOpen} open={open}>
      <DialogContent className="max-w-0 min-w-[650px] md:max-w-[70svw] min-h-[500px] max-h-[90svh] lg:max-w-[850px] flex flex-col p-0">
        <DialogHeader className="p-6 pb-4 shrink-0 border-b">
          <DialogTitle className="text-2xl font-semibold mb-1">{addon.alias}</DialogTitle>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground">
            <span className="inline-flex items-center gap-1">
              <User className="w-3.5 h-3.5" />
              <span className="font-normal text-foreground/90">{addon.author}</span>
            </span>
            {ReleaseInformation()}
          </div>
          {addon.description && (
            <DialogDescription className="text-sm text-muted-foreground">
              {addon.description}
            </DialogDescription>
          )}
        </DialogHeader>

        <div className="flex-1 overflow-y-auto p-6">
          {release ? (
            <>
              <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground mb-4">
                <CalendarDays className="w-3.5 h-3.5" />
                <span>Released on {formatToLocalTime(release.published_at)}</span>
              </div>
              <div className="border rounded-lg p-4 bg-card">
                <div className="prose max-w-none text-sm text-foreground dark:text-foreground/90">
                  <Markdown>{changelog}</Markdown>
                </div>
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
              <Package className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">No changelog available</p>
              <p className="text-xs mt-1">The author hasn't provided release notes.</p>
            </div>
          )}
        </div>

        <DialogFooter className="p-4 border-t">TODO: Add buttons</DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
