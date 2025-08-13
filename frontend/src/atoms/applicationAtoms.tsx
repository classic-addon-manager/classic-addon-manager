import { atomWithDefault } from 'jotai/utils'

import { ApplicationService } from '@/lib/wails'

export const versionAtom = atomWithDefault<Promise<string>>(async () => {
  const version = await ApplicationService.GetVersion()
  return version || '1.0.0'
})
