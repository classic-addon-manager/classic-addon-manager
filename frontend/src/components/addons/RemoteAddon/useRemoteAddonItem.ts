import { useSetAtom } from 'jotai'
import { useState } from 'react'

import { isManifestDialogOpenAtom, selectedManifestAtom } from '@/components/addons/atoms'
import { daysAgo } from '@/lib/utils'

import type { RemoteAddonItemProps } from './types'

export const useRemoteAddonItem = ({ manifest, installed }: RemoteAddonItemProps) => {
  const [hasIcon, setIcon] = useState(true)
  const setSelectedManifest = useSetAtom(selectedManifestAtom)
  const setDialogOpen = useSetAtom(isManifestDialogOpenAtom)

  const isNew = daysAgo(manifest.added_at) < 32
  const iconUrl = `https://raw.githubusercontent.com/${manifest.repo}/${manifest.branch}/icon.png`

  const openDialog = () => {
    setSelectedManifest(manifest)
    setDialogOpen(true)
  }

  return {
    manifest,
    installed,
    hasIcon,
    setIcon,
    isNew,
    iconUrl,
    openDialog,
  }
}
