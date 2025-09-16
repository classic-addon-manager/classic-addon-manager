import { Dialogs } from '@wailsio/runtime'
import { useAtom, useAtomValue } from 'jotai'
import { AlertTriangleIcon, LoaderCircle, Package, RefreshCw, Search, Upload } from 'lucide-react'
import { Suspense, useEffect, useState } from 'react'
import { useDebouncedCallback } from 'use-debounce'

import { AddonList } from '@/components/dashboard/AddonList'
import {
  filteredAddonsAtom,
  isAddonDialogOpenAtom,
  searchQueryAtom,
  selectedAddonAtom,
  versionSelectAtom,
} from '@/components/dashboard/atoms'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast.tsx'
import { LocalAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

import { LocalAddonDialog, LocalAddonVersionSelectDialog } from './dashboard/index'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from './ui/tooltip'

export const Dashboard = () => {
  const { isCheckingForUpdates, performBulkUpdateCheck, updateInstalledAddons } = useAddonStore()
  const [isLoading, setIsLoading] = useState(false)
  const [selectedAddon, setSelectedAddon] = useAtom(selectedAddonAtom)
  const [isAddonDialogOpen, setIsAddonDialogOpen] = useAtom(isAddonDialogOpenAtom)
  const [, setSearchQuery] = useAtom(searchQueryAtom)
  const filteredAddons = useAtomValue(filteredAddonsAtom)
  const versionSelectAddon = useAtomValue(versionSelectAtom)

  const debouncedSetSearch = useDebouncedCallback((value: string) => {
    setSearchQuery(value)
  }, 300)

  useEffect(() => {
    const loadAddons = async () => {
      setIsLoading(true)
      await updateInstalledAddons()
      setIsLoading(false)
    }

    loadAddons().then(() => {
      performBulkUpdateCheck()
    })
  }, [])

  const handleInstallZip = async () => {
    try {
      const selectedFile = await Dialogs.OpenFile({
        Title: 'Select Addon ZIP File',
        Message: 'Choose a ZIP file containing the addon to install',
        ButtonText: 'Install',
        CanChooseFiles: true,
        CanChooseDirectories: false,
        AllowsMultipleSelection: false,
        Filters: [
          {
            DisplayName: 'ZIP Files',
            Pattern: '*.zip',
          },
        ],
      })

      if (selectedFile) {
        const name = await LocalAddonService.InstallZipAddon(selectedFile)
        toast({
          title: 'Addon Installed',
          description: `${name} installed successfully!`,
        })
        // Refresh the addon list after installation
        await updateInstalledAddons()
      }
    } catch (error: unknown) {
      if (error instanceof Error) {
        // Check if dialog was cancelled without making a selection
        if (error.message.includes('shellItem is nil')) {
          return
        }
        toast({
          title: 'Error',
          description: error.message,
          icon: AlertTriangleIcon,
        })
        return
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

  return (
    <div className="flex flex-col h-screen bg-background">
      <header className="sticky top-0 z-10 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
        <div className="container mx-auto flex h-16 items-center gap-4 px-4">
          <div className="flex-1">
            <div className="relative w-full">
              <Search className="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2" />
              <Input
                placeholder="Search installed addons"
                type="search"
                className="pl-10 pr-4 w-full transition-colors focus-visible:ring-1"
                disabled={isCheckingForUpdates}
                onChange={event => debouncedSetSearch(event.target.value)}
              />
            </div>
          </div>

          <div className="flex items-center justify-center">
            <Button
              variant="outline"
              onClick={() => {
                performBulkUpdateCheck()
              }}
              disabled={isCheckingForUpdates}
              className="transition-all duration-200 hover:shadow-md min-w-[160px]"
            >
              {isCheckingForUpdates ? (
                <>
                  <LoaderCircle className="mr-2 size-4 animate-spin" />
                  Checking...
                </>
              ) : (
                <>
                  <RefreshCw className="mr-2 size-4" />
                  Check Updates
                </>
              )}
            </Button>
          </div>
        </div>
      </header>

      {selectedAddon && (
        <LocalAddonDialog
          addon={selectedAddon}
          open={isAddonDialogOpen}
          onOpenChange={open => {
            setIsAddonDialogOpen(open)
            if (!open) {
              setTimeout(() => setSelectedAddon(null), 200)
            }
          }}
        />
      )}

      {versionSelectAddon && <LocalAddonVersionSelectDialog addon={versionSelectAddon} />}

      <main className="flex-1 overflow-auto">
        <div className="container mx-auto px-4 pt-4">
          <div className="flex items-center justify-between mb-6">
            <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
            {!isLoading && (
              <div className="flex items-center gap-3">
                <TooltipProvider>
                  <Tooltip delayDuration={100}>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={handleInstallZip}
                        className="flex items-center gap-2 transition-all duration-200 hover:shadow-md"
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

                <Badge
                  variant="secondary"
                  className="flex text-muted-foreground items-center gap-1 px-2 py-1.5"
                >
                  <Package className="size-3" />
                  {filteredAddons.length} {filteredAddons.length === 1 ? 'addon' : 'addons'}{' '}
                  installed
                </Badge>
              </div>
            )}
          </div>

          <Suspense
            fallback={
              <div className="flex flex-col items-center justify-center rounded-lg border bg-card/50 min-h-screen-minus-12 relative overflow-hidden">
                <div className="absolute inset-0 bg-grid-white/[0.02] bg-[size:16px_16px]" />
                <div className="relative z-10">
                  <div className="absolute -inset-8 rounded-full bg-primary/10 blur-xl animate-pulse" />
                  <div className="absolute -inset-4 rounded-full bg-primary/20 animate-ping opacity-75" />
                  <LoaderCircle
                    className="size-12 animate-spin text-primary relative"
                    strokeWidth={1.5}
                  />
                </div>
                <p className="text-muted-foreground mt-6 relative z-10 animate-pulse">
                  Loading your addons...
                </p>
              </div>
            }
          >
            <AddonList />
          </Suspense>
        </div>
      </main>
    </div>
  )
}
