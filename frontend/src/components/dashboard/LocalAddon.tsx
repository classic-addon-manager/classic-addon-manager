import { cn } from '@/lib/utils'
import { Addon } from '@/lib/wails'
import { AddonStatus } from '@/components/dashboard/AddonStatus.tsx'
import { LocalAddonContextMenu } from '@/components/dashboard/LocalAddonContextMenu.tsx'
import { useState } from 'react'
import { LocalAddonDialog } from '@/components/dashboard/LocalAddonDialog'

interface LocalAddonProps {
  addon: Addon
}

export const LocalAddon = ({ addon }: LocalAddonProps) => {
  const [isContextMenuOpen, setIsContextMenuOpen] = useState(false)
  const [isDialogOpen, setIsDialogOpen] = useState(false)
  return (
    <>
      <LocalAddonDialog addon={addon} open={isDialogOpen} onOpenChange={setIsDialogOpen} />
      <LocalAddonContextMenu addon={addon} onOpenChange={setIsContextMenuOpen}>
        <div
          className={cn(
            'cursor-pointer grid grid-cols-4 p-2 hover:bg-muted/50 border-t transition-colors items-center text-sm',
            isContextMenuOpen && 'bg-muted'
          )}
          onClick={() => setIsDialogOpen(true)}
        >
          <div className="font-medium">{addon.alias}</div>
          <div className="text-center">{addon.author}</div>
          <div className="text-center">{addon.version}</div>
          <div className="text-center">
            <AddonStatus addon={addon} />
          </div>
        </div>
      </LocalAddonContextMenu>
    </>
  )
}
