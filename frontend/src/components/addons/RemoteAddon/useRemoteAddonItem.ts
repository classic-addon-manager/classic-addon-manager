import { useSetAtom } from 'jotai'

import { isManifestDialogOpenAtom, selectedManifestAtom } from '@/components/addons/atoms'
import { addonIconUrl } from '@/lib/icon'
import { isNewAddon } from '@/lib/utils'

import type { RemoteAddonItemProps } from './types'

export const useRemoteAddonItem = ({ manifest, installed }: RemoteAddonItemProps) => {
  const setSelectedManifest = useSetAtom(selectedManifestAtom)
  const setDialogOpen = useSetAtom(isManifestDialogOpenAtom)

  const isNew = isNewAddon(manifest.added_at)
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
