import { AlertTriangleIcon } from 'lucide-react'

import { toast } from '@/components/ui/toast'
import type { InstallWithDependenciesResult } from '@/lib/wails'

export function notifyDependencyResult(
  result: InstallWithDependenciesResult,
  parentFailureFallback: string
): boolean {
  if (result.dependencyWarnings.length > 0) {
    toast({
      title: 'Dependency resolution warning',
      description: result.dependencyWarnings[0],
      icon: AlertTriangleIcon,
    })
  }

  result.dependencies
    .filter(dep => dep.success && !dep.skipped)
    .forEach(dep => {
      toast({
        title: 'Dependency installed',
        description: `${dep.alias} installed successfully`,
      })
    })

  if (!result.success) {
    const failedDeps = result.dependencies.filter(dep => !dep.success)
    if (failedDeps.length > 0) {
      toast({
        title: 'Dependency installation failed',
        description: `Failed: ${failedDeps.map(dep => dep.alias || dep.name).join(', ')}`,
        icon: AlertTriangleIcon,
      })
    } else {
      toast({
        title: 'Error',
        description: result.mainAddon.error || parentFailureFallback,
        icon: AlertTriangleIcon,
      })
    }
  }

  return result.success
}
