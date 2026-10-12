import { AlertTriangleIcon } from 'lucide-react'

import { toast } from '@/components/ui/toast'
import type { InstallWithDependenciesResult } from '@/lib/wails'

export interface DependencyFailure {
  /** Whether a dependency failed, or the parent addon itself. */
  kind: 'dependencies' | 'addon'
  description: string
}

/** Toasts dependency warnings and each dependency that was newly installed. */
export function notifyDependencyProgress(result: InstallWithDependenciesResult): void {
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
}

/** Describes why an install failed, or null when it succeeded. */
export function dependencyFailure(
  result: InstallWithDependenciesResult,
  parentFailureFallback: string
): DependencyFailure | null {
  if (result.success) return null

  const failedDeps = result.dependencies.filter(dep => !dep.success)
  if (failedDeps.length > 0) {
    return {
      kind: 'dependencies',
      description: `Failed: ${failedDeps.map(dep => dep.alias || dep.name).join(', ')}`,
    }
  }

  return {
    kind: 'addon',
    description: result.mainAddon.error || parentFailureFallback,
  }
}

export function notifyDependencyResult(
  result: InstallWithDependenciesResult,
  parentFailureFallback: string
): boolean {
  notifyDependencyProgress(result)

  const failure = dependencyFailure(result, parentFailureFallback)
  if (failure) {
    toast({
      title: failure.kind === 'dependencies' ? 'Dependency installation failed' : 'Error',
      description: failure.description,
      icon: AlertTriangleIcon,
    })
  }

  return failure === null
}
