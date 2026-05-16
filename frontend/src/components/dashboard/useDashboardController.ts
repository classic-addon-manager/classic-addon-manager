import { Dialogs } from '@wailsio/runtime'
import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import { AlertTriangleIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useDebouncedCallback } from 'use-debounce'

import {
  filteredAddonsAtom,
  searchQueryAtom,
  selectedAddonAtom,
  versionSelectAtom,
} from '@/components/dashboard/atoms'
import { toast } from '@/components/ui/toast.tsx'
import { LocalAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

export function useDashboardController() {
  const { installedAddons, isCheckingForUpdates, performBulkUpdateCheck, updateInstalledAddons } =
    useAddonStore()
  const [isLoading, setIsLoading] = useState(false)
  const [selectedAddon, setSelectedAddon] = useAtom(selectedAddonAtom)
  const setSearchQuery = useSetAtom(searchQueryAtom)
  const filteredAddons = useAtomValue(filteredAddonsAtom)
  const versionSelectAddon = useAtomValue(versionSelectAtom)

  useEffect(() => {
    if (!selectedAddon) return
    const fresh = installedAddons.find(a => a.name === selectedAddon.name)
    if (fresh && fresh !== selectedAddon) setSelectedAddon(fresh)
  }, [installedAddons, selectedAddon, setSelectedAddon])

  const storeActionsRef = useRef({ updateInstalledAddons, performBulkUpdateCheck })
  storeActionsRef.current = { updateInstalledAddons, performBulkUpdateCheck }

  const debouncedSetSearch = useDebouncedCallback((value: string) => {
    setSearchQuery(value)
  }, 300)

  useEffect(() => {
    const loadAddons = async () => {
      setIsLoading(true)
      await storeActionsRef.current.updateInstalledAddons()
      setIsLoading(false)
    }

    loadAddons().then(() => {
      storeActionsRef.current.performBulkUpdateCheck()
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
        await storeActionsRef.current.updateInstalledAddons()
      }
    } catch (error: unknown) {
      if (error instanceof Error) {
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

  return {
    isLoading,
    isCheckingForUpdates,
    filteredAddons,
    selectedAddon,
    setSelectedAddon,
    versionSelectAddon,
    debouncedSetSearch,
    performBulkUpdateCheck,
    handleInstallZip,
  }
}
