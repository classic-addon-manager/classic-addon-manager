import { HardDrive, Settings } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast.tsx'
import { ApplicationService } from '@/lib/wails'

export const QuickActions = () => {
  const handleOpenDataLocation = async () => {
    try {
      await ApplicationService.OpenDataDir()
    } catch (error: unknown) {
      toast({
        title: 'Error',
        description: error instanceof Error ? error.message : 'Failed to open data location',
      })
    }
  }

  const handleOpenCacheLocation = async () => {
    try {
      await ApplicationService.OpenCacheDir()
    } catch (error: unknown) {
      toast({
        title: 'Error',
        description: error instanceof Error ? error.message : 'Failed to open cache location',
      })
    }
  }

  return (
    <div className="flex items-center gap-2">
      <Button
        variant="outline"
        size="sm"
        onClick={handleOpenCacheLocation}
        className="flex items-center gap-2"
      >
        <HardDrive className="h-4 w-4" />
        Open Cache Location
      </Button>
      <Button
        variant="outline"
        size="sm"
        onClick={handleOpenDataLocation}
        className="flex items-center gap-2"
      >
        <Settings className="h-4 w-4" />
        Open Data Location
      </Button>
    </div>
  )
}
