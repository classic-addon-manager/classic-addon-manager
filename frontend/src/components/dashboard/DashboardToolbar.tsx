import { Dialogs } from '@wailsio/runtime'
import { useSetAtom } from 'jotai'
import { AlertTriangleIcon, LoaderCircle, Package, RefreshCw, Search, Upload } from 'lucide-react'
import { useDebouncedCallback } from 'use-debounce'

import { searchQueryAtom } from '@/components/dashboard/atoms'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useTitleBarSlot } from '@/hooks/useTitleBarSlot'
import { LocalAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

export const DashboardToolbar = () => {
  const setSearchQuery = useSetAtom(searchQueryAtom)

  const {
    installedAddons,
    updatesAvailableCount,
    isCheckingForUpdates,
    performBulkUpdateCheck,
    updateInstalledAddons,
  } = useAddonStore()

  const debouncedSetSearch = useDebouncedCallback((value: string) => {
    setSearchQuery(value)
  }, 300)

  const handleInstallZip = async () => {
    try {
      const selectedFile = await Dialogs.OpenFile({
        Title: 'Select Addon ZIP File',
        Message: 'Choose a ZIP file containing the addon to install',
        ButtonText: 'Install',
        CanChooseFiles: true,
        CanChooseDirectories: false,
        AllowsMultipleSelection: false,
        Filters: [{ DisplayName: 'ZIP Files', Pattern: '*.zip' }],
      })

      if (selectedFile) {
        const name = await LocalAddonService.InstallZipAddon(selectedFile)
        toast({
          title: 'Addon Installed',
          description: `${name} installed successfully!`,
        })
        await updateInstalledAddons()
      }
    } catch (error: unknown) {
      if (error instanceof Error) {
        if (error.message.includes('shellItem is nil')) return
        toast({
          title: 'Error',
          description: error.message,
          icon: AlertTriangleIcon,
        })
      } else {
        toast({
          title: 'Error',
          description: 'An unknown error occurred: ' + error,
          icon: AlertTriangleIcon,
        })
        console.error('Error selecting ZIP file:', error)
      }
    }
  }

  const addonCount = installedAddons.length

  const toolbarContent = (
    <div className="flex w-full min-w-0 items-center gap-3">
      <div className="no-drag relative min-w-0 flex-1">
        <Search className="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2" />
        <Input
          placeholder="Search installed addons"
          type="search"
          className="pl-10 pr-4 w-full transition-colors focus-visible:ring-1"
          disabled={isCheckingForUpdates}
          onChange={event => debouncedSetSearch(event.target.value)}
        />
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <div className="no-drag flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => performBulkUpdateCheck()}
            disabled={isCheckingForUpdates}
            className="transition-all duration-200 hover:shadow-md h-8"
          >
            {isCheckingForUpdates ? (
              <>
                <LoaderCircle className="mr-1.5 size-3.5 animate-spin" />
                Refresh
              </>
            ) : (
              <>
                <RefreshCw className="mr-1.5 size-3.5" />
                Refresh
              </>
            )}
          </Button>

          <TooltipProvider>
            <Tooltip delayDuration={100}>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleInstallZip}
                  className="flex items-center gap-1.5 transition-all duration-200 hover:shadow-md h-8"
                >
                  <Upload className="size-3.5" />
                  Install ZIP
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p className="flex gap-2">
                  <AlertTriangleIcon className="size-4" />
                  Only install addons from sources you trust!
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>

        {addonCount > 0 && (
          <Badge
            variant="secondary"
            className="flex text-muted-foreground items-center gap-1 px-2 py-1 text-xs"
          >
            <Package className="size-3" />
            {addonCount} {addonCount === 1 ? 'addon' : 'addons'}
          </Badge>
        )}

        {updatesAvailableCount > 0 && (
          <Badge
            variant="outline"
            className="flex items-center gap-1 px-2 py-1 text-xs text-primary border-primary/20 bg-primary/10"
          >
            <RefreshCw className="size-3" />
            {updatesAvailableCount} update{updatesAvailableCount !== 1 ? 's' : ''}
          </Badge>
        )}
      </div>
    </div>
  )

  useTitleBarSlot(toolbarContent)

  return null
}
