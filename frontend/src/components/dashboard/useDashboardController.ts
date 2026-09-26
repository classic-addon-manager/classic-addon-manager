import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import { useEffect, useState } from 'react'

import {
  filteredAddonsAtom,
  selectedAddonAtom,
  versionSelectAtom,
} from '@/components/dashboard/atoms'
import { loadCatalogIconMapAtom } from '@/components/dashboard/iconMap'
import { useInstallZipAddon } from '@/lib/addon'
import { useAddonStore } from '@/stores/addonStore'

export function useDashboardController() {
  const loadCatalogIcons = useSetAtom(loadCatalogIconMapAtom)
  const { installedAddons, isCheckingForUpdates, performBulkUpdateCheck, updateInstalledAddons } =
    useAddonStore()
  const [isLoading, setIsLoading] = useState(true)
  const [selectedAddonSnapshot, setSelectedAddon] = useAtom(selectedAddonAtom)
  const filteredAddons = useAtomValue(filteredAddonsAtom)
  const versionSelectAddon = useAtomValue(versionSelectAtom)

  const selectedAddon = selectedAddonSnapshot
    ? (installedAddons.find(addon => addon.name === selectedAddonSnapshot.name) ??
      selectedAddonSnapshot)
    : null

  useEffect(() => {
    const loadAddons = async () => {
      await updateInstalledAddons()
      setIsLoading(false)
    }

    loadAddons().then(() => {
      performBulkUpdateCheck()
    })
    loadCatalogIcons()
  }, [updateInstalledAddons, performBulkUpdateCheck, loadCatalogIcons])

  const { installZip } = useInstallZipAddon()

  return {
    isLoading,
    isCheckingForUpdates,
    filteredAddons,
    selectedAddon,
    setSelectedAddon,
    versionSelectAddon,
    performBulkUpdateCheck,
    handleInstallZip: installZip,
  }
}
