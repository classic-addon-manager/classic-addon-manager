import { atom } from 'jotai'
import { atomWithDefault } from 'jotai/utils'

import type { ApplicationRelease } from '@/lib/wails'
import { ApplicationService } from '@/lib/wails'

export const versionAtom = atomWithDefault<Promise<string>>(async () => {
  const version = await ApplicationService.GetVersion()
  return version || '1.0.0'
})

export const updateAvailableAtom = atom(false)
export const updateInformationAtom = atom<ApplicationRelease | null>(null)
export const updateDialogOpenAtom = atom(false)
