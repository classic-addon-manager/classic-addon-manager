import { Browser } from '@wailsio/runtime'
import {
  AlertTriangleIcon,
  BugIcon,
  CheckIcon,
  FolderOpen,
  GithubIcon,
  Trash2Icon,
} from 'lucide-react'
import type { ReactNode } from 'react'

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'
import { toast } from '@/components/ui/toast.tsx'
import { safeCall } from '@/lib/utils.ts'
import type { Addon } from '@/lib/wails'
import { LocalAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore.ts'

interface LocalAddonContextMenuProps {
  addon: Addon
  children: ReactNode
  onOpenChange: (open: boolean) => void
}

export const LocalAddonContextMenu = ({
  addon,
  children,
  onOpenChange,
}: LocalAddonContextMenuProps) => {
  const { uninstall, unmanage } = useAddonStore()

  const openDirectory = async () => {
    const [, err] = await safeCall(LocalAddonService.OpenDirectory(addon.name))
    if (err) {
      console.error('Failed to open directory:', err)
      toast({
        icon: AlertTriangleIcon,
        title: 'Error',
        description: `Failed to open directory for addon "${addon.alias}".`,
      })
    }
  }

  const handleAddonAction = async (action: 'uninstall' | 'unmanage') => {
    const actionPast = action === 'uninstall' ? 'Uninstalled' : 'Unmanaged'

    const [result, err] = await safeCall(
      action === 'uninstall' ? uninstall(addon) : unmanage(addon)
    )

    if (err) {
      console.error(`[LocalAddonContextMenu] Failed to ${action} addon:`, err)
      toast({
        icon: AlertTriangleIcon,
        title: 'Error',
        description: `Failed to ${action} addon "${addon.alias}".`,
      })

      return
    }

    if (result) {
      toast({
        icon: CheckIcon,
        title: actionPast,
        description: `Addon "${addon.alias}" has been ${actionPast.toLowerCase()}.`,
      })
    } else {
      toast({
        icon: AlertTriangleIcon,
        title: 'Error',
        description: `Failed to ${action} addon "${addon.alias}".`,
      })
    }
  }

  return (
    <ContextMenu onOpenChange={onOpenChange}>
      <ContextMenuTrigger>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={openDirectory}>
          <FolderOpen size={16} />
          Open directory
        </ContextMenuItem>
        {addon.isManaged && (
          <>
            <ContextMenuItem onClick={() => Browser.OpenURL(`https://github.com/${addon.repo}`)}>
              <GithubIcon size={16} />
              <span>View code</span>
            </ContextMenuItem>

            <ContextMenuItem
              onClick={() => Browser.OpenURL(`https://github.com/${addon.repo}/issues/new`)}
            >
              <BugIcon size={16} />
              <span>Report issue</span>
            </ContextMenuItem>

            <ContextMenuItem variant="warning" onClick={() => handleAddonAction('unmanage')}>
              <AlertTriangleIcon size={16} />
              <span>Unmanage</span>
            </ContextMenuItem>
          </>
        )}
        <ContextMenuItem variant="destructive" onClick={() => handleAddonAction('uninstall')}>
          <Trash2Icon size={16} />
          Uninstall
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}
