import { useAtomValue } from 'jotai'
import { Download, LoaderCircle, Sparkles } from 'lucide-react'
import { useEffect, useState } from 'react'
import semver from 'semver'

import { versionAtom } from '@/atoms/applicationAtoms'
import { Card } from '@/components/ui/card'
import type { ApplicationRelease } from '@/lib/wails'
import { ApplicationService } from '@/lib/wails'

export const UpdateCard = () => {
  const currentVersion = useAtomValue(versionAtom)
  const [updateAvailable, setUpdateAvailable] = useState(false)
  const [updateInformation, setUpdateInformation] = useState<ApplicationRelease | null>(null)
  const [isUpdating, setIsUpdating] = useState(false)

  useEffect(() => {
    const checkForUpdates = async () => {
      try {
        const releaseInfo = await ApplicationService.GetLatestRelease()
        if (releaseInfo && semver.gt(releaseInfo.version, currentVersion)) {
          setUpdateInformation(releaseInfo)
          setUpdateAvailable(true)
        }
      } catch (error) {
        console.error('Failed to get latest release:', error)
      }
    }

    checkForUpdates()
  }, [currentVersion])

  const handleUpdate = async () => {
    if (!updateInformation || isUpdating) return

    setIsUpdating(true)
    try {
      await ApplicationService.SelfUpdate(updateInformation.url)
    } catch (error) {
      console.error('Update failed:', error)
      setIsUpdating(false)
    }
  }

  if (!updateAvailable || !updateInformation) {
    return null
  }

  return (
    <div className="px-2 mt-auto select-none">
      <Card
        role="button"
        aria-label={isUpdating ? 'Updating application' : 'Update available'}
        onClick={handleUpdate}
        className={[
          'relative group flex items-center gap-2 rounded-xl px-3 py-2 cursor-pointer',
          'h-30 overflow-hidden', // fixed stable height
          'border border-transparent bg-gradient-to-r from-indigo-600 via-fuchsia-600 to-violet-600',
          'shadow-md shadow-indigo-900/20 transition-all',
          'hover:shadow-lg',
          isUpdating ? 'opacity-80' : 'hover:brightness-105',
        ].join(' ')}
      >
        {/* Inner surface */}
        <div className="absolute inset-[1px] rounded-[11px] bg-zinc-900/80 backdrop-blur-sm" />

        {/* Subtle moving sheen */}
        <div className="pointer-events-none absolute inset-0 overflow-hidden rounded-xl">
          <div className="absolute -inset-[40%] animate-[spin_12s_linear_infinite] bg-[conic-gradient(var(--tw-gradient-stops))] from-transparent via-white/5 to-transparent opacity-60" />
        </div>

        {/* Icon / status */}
        <div className="relative flex items-center justify-center w-6 h-6 rounded-lg bg-gradient-to-br from-indigo-500/30 to-fuchsia-500/30 text-fuchsia-200 ring-1 ring-white/10">
          {isUpdating ? (
            <LoaderCircle className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Sparkles className="w-3.5 h-3.5 animate-pulse" />
          )}
        </div>

        {/* Text */}
        <div className="relative flex flex-col leading-tight min-w-0 flex-1" aria-live="polite">
          <span className="text-xs font-medium tracking-wide text-white/90 truncate whitespace-nowrap">
            {isUpdating ? 'Updating…' : 'Update Ready'}
          </span>
          <span className="text-xs text-white/50 font-mono truncate whitespace-nowrap">
            {currentVersion.replace(/^v/, '')} → {updateInformation.version.replace(/^v/, '')}
          </span>
          <span className="text-xs text-white/60 truncate whitespace-nowrap">
            {isUpdating ? '' : 'Click to install'}
          </span>
        </div>

        {/* Action */}
        <div className="relative ml-auto flex items-center pl-1 w-10 justify-end flex-shrink-0">
          {isUpdating ? (
            <div className="flex items-center gap-1 w-full justify-end">
              <span className="inline-block h-1 w-8 overflow-hidden rounded-full bg-white/10">
                <span className="block h-full w-full origin-left animate-[pulse_1.2s_ease-in-out_infinite] bg-gradient-to-r from-indigo-400 via-fuchsia-400 to-violet-400" />
              </span>
            </div>
          ) : (
            <div className="flex items-center justify-end w-full text-[10px] font-semibold text-white/90">
              <Download className="w-3 h-3" />
            </div>
          )}
        </div>

        {/* Focus ring */}
        <span className="absolute inset-0 rounded-xl ring-2 ring-transparent ring-offset-0 focus-visible:ring-fuchsia-400 focus:outline-none" />
      </Card>
    </div>
  )
}
