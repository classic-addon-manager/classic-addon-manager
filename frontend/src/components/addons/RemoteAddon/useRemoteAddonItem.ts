import { useSetAtom } from 'jotai'
import { useEffect, useRef } from 'react'

import { isManifestDialogOpenAtom, selectedManifestAtom } from '@/components/addons/atoms'
import { prefetchAddonDetails } from '@/lib/addonQueries'
import { addonIconUrl } from '@/lib/icon'
import { isNewAddon } from '@/lib/utils'

import type { RemoteAddonItemProps } from './types'

const PREFETCH_DELAY_MS = 150

export const useRemoteAddonItem = ({ manifest, installed }: RemoteAddonItemProps) => {
  const setSelectedManifest = useSetAtom(selectedManifestAtom)
  const setDialogOpen = useSetAtom(isManifestDialogOpenAtom)

  const prefetchTimer = useRef<ReturnType<typeof setTimeout>>(undefined)
  useEffect(() => () => clearTimeout(prefetchTimer.current), [])

  // Wait briefly so sweeping the pointer down the list doesn't prefetch every row it crosses.
  const startPrefetch = () => {
    clearTimeout(prefetchTimer.current)
    prefetchTimer.current = setTimeout(() => prefetchAddonDetails(manifest), PREFETCH_DELAY_MS)
  }
  const cancelPrefetch = () => clearTimeout(prefetchTimer.current)

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
    prefetchHandlers: {
      onPointerEnter: startPrefetch,
      onPointerLeave: cancelPrefetch,
    },
  }
}
