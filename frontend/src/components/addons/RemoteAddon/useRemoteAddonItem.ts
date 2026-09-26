import { useSetAtom } from 'jotai'

import { isManifestDialogOpenAtom, selectedManifestAtom } from '@/components/addons/atoms'
import { addonIconUrl } from '@/lib/icon'
import { daysAgo, NEW_ADDON_DAYS } from '@/lib/utils'

import type { RemoteAddonItemProps } from './types'

export const useRemoteAddonItem = ({ manifest, installed }: RemoteAddonItemProps) => {
  const setSelectedManifest = useSetAtom(selectedManifestAtom)
  const setDialogOpen = useSetAtom(isManifestDialogOpenAtom)

  const isNew = daysAgo(manifest.added_at) < NEW_ADDON_DAYS
  const iconUrl = addonIconUrl(manifest)

  const openDialog = () => {
    setSelectedManifest(manifest)
    setDialogOpen(true)
  }

  return {
    manifest,
    installed,
    isNew,
    iconUrl,
    openDialog,
  }
}
