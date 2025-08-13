import { useAtomValue } from 'jotai'
import { Download, LoaderCircle, Sparkles } from 'lucide-react'
import { useEffect, useState } from 'react'

import { versionAtom } from '@/atoms/applicationAtoms'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import type { ApplicationRelease } from '@/lib/wails'
import { ApplicationService } from '@/lib/wails'

function isNewerVersion(newVersion: string, currentVersion: string): boolean {
  const parseVersion = (version: string) => {
    const cleaned = version.replace(/^v/, '') // Remove 'v' prefix if present
    return cleaned.split('.').map(num => parseInt(num, 10))
  }

  const newVer = parseVersion(newVersion)
  const currentVer = parseVersion(currentVersion)

  for (let i = 0; i < Math.max(newVer.length, currentVer.length); i++) {
    const newPart = newVer[i] || 0
    const currentPart = currentVer[i] || 0

    if (newPart > currentPart) return true
    if (newPart < currentPart) return false
  }

  return false
}

export const UpdateCard = () => {
  const currentVersion = useAtomValue(versionAtom)
  const [updateAvailable, setUpdateAvailable] = useState(false)
  const [updateInformation, setUpdateInformation] = useState<ApplicationRelease | null>(null)
  const [isUpdating, setIsUpdating] = useState(false)

  useEffect(() => {
    const checkForUpdates = async () => {
      try {
        const releaseInfo = await ApplicationService.GetLatestRelease()
        if (releaseInfo && isNewerVersion(releaseInfo.version, currentVersion)) {
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
    if (!updateInformation) return

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
    <div className="px-2 mt-auto">
      <Card className="relative overflow-hidden border-0 bg-gradient-to-br from-blue-600 to-purple-700 text-white shadow-lg">
        <div className="absolute inset-0 bg-gradient-to-br from-blue-500/20 to-purple-600/20 backdrop-blur-sm" />
        <div className="relative p-3">
          <div className="flex items-center gap-2 mb-2">
            <div className="flex items-center justify-center w-6 h-6 rounded-full bg-white/20">
              <Sparkles className="w-3 h-3" />
            </div>
            <div className="flex-1">
              <h3 className="text-sm font-semibold leading-tight">Update Ready!</h3>
              <p className="text-xs text-white/80">{updateInformation.version}</p>
            </div>
          </div>

          <Button
            size="sm"
            variant="secondary"
            className="w-full h-8 text-xs bg-white/90 hover:bg-white text-gray-900 shadow-sm"
            onClick={handleUpdate}
            disabled={isUpdating}
          >
            {isUpdating ? (
              <>
                <LoaderCircle className="w-3 h-3 mr-1.5 animate-spin" />
                Updating...
              </>
            ) : (
              <>
                <Download className="w-3 h-3 mr-1.5" />
                Update Now
              </>
            )}
          </Button>
        </div>
      </Card>
    </div>
  )
}
