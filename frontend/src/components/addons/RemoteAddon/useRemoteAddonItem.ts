import { useSetAtom } from 'jotai'
import { useState } from 'react'

import { isManifestDialogOpenAtom, selectedManifestAtom } from '@/components/addons/atoms'
import { addonIconUrl } from '@/lib/icon'
import { daysAgo, NEW_ADDON_DAYS } from '@/lib/utils'

import type { RemoteAddonItemProps } from './types'

export const useRemoteAddonItem = ({ manifest, installed }: RemoteAddonItemProps) => {
  const [failedIconUrl, setFailedIconUrl] = useState<string | null>(null)
  const setSelectedManifest = useSetAtom(selectedManifestAtom)
  const setDialogOpen = useSetAtom(isManifestDialogOpenAtom)

  const isNew = daysAgo(manifest.added_at) < NEW_ADDON_DAYS
  const iconUrl = addonIconUrl(manifest)
  const hasIcon = iconUrl !== null && failedIconUrl !== iconUrl

  const openDialog = () => {
    setSelectedManifest(manifest)
    setDialogOpen(true)
  }

  return {
    manifest,
    installed,
    hasIcon,
    isNew,
    iconUrl,
    openDialog,
    onIconError: () => {
      if (iconUrl) setFailedIconUrl(iconUrl)
    },
  }
}
