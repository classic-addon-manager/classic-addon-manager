import { AlertTriangleIcon } from 'lucide-react'
import { useCallback, useState } from 'react'

import { toast } from '@/components/ui/toast.tsx'
import { rateAddon } from '@/lib/addon.ts'
import { apiClient } from '@/lib/api.ts'
import {
  type DependencyInfo,
  installDependenciesInOrder,
  resolveTransitiveDependencies,
} from '@/lib/dependency-resolver.ts'
import { safeCall } from '@/lib/utils.ts'
import type { AddonManifest, Release } from '@/lib/wails'
import { LocalAddonService, RemoteAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore.ts'
import { useUserStore } from '@/stores/userStore.ts'

interface UseAddonActionsProps {
  manifest: AddonManifest
  onViewDependency: (manifest: AddonManifest) => void
  onOpenChange: (open: boolean) => void
  onAddonInstalled?: () => void
  onAddonUninstalled?: () => void
}

export const useAddonActions = ({
  manifest,
  onViewDependency,
  onOpenChange,
  onAddonInstalled,
  onAddonUninstalled,
}: UseAddonActionsProps) => {
  const [release, setRelease] = useState<Release | null>(null)
  const [readme, setReadme] = useState<string>('Loading description...')
  const [changelog, setChangelog] = useState<string>('Loading changelog...')
  const [rating, setRating] = useState(0)
  const [dependencies, setDependencies] = useState<DependencyInfo[]>([])
  const { isAuthenticated } = useUserStore()
  const [isInstalled, setIsInstalled] = useState<boolean>(false)
  const [isProcessing, setIsProcessing] = useState<boolean>(false)

  const checkInstalledStatus = useCallback(async () => {
    const [installed] = await safeCall(LocalAddonService.IsInstalled(manifest.name))
    setIsInstalled(!!installed)
  }, [manifest.name])

  const handleInstall = async () => {
    if (isProcessing) return

    if (isInstalled) {
      toast({
        title: 'Already installed',
        description: `${manifest.alias} is already installed.`,
      })
      return
    }

    if (!release) {
      toast({
        title: 'Error',
        description: 'Cannot install addon without a release.',
        icon: AlertTriangleIcon,
      })
      return
    }

    setIsProcessing(true)
    let didInstall = false

    try {
      // Handle dependencies if any
      if (dependencies.length > 0) {
        const uninstalledDeps = dependencies.filter(d => !d.isInstalled)

        if (uninstalledDeps.length > 0) {
          const failedDepNames = await installDependenciesInOrder(dependencies)

          if (failedDepNames.length > 0) {
            const failedDepAliases = failedDepNames
              .map(name => {
                const dep = dependencies.find(d => d.manifest.name === name)
                return dep ? dep.manifest.alias : name
              })
              .join(', ')

            toast({
              title: 'Dependency installation failed',
              description: `Failed: ${failedDepAliases}`,
              icon: AlertTriangleIcon,
            })
            setIsProcessing(false)
            return
          }

          // Show a success toast for each dependency that was installed
          uninstalledDeps.forEach(depInfo => {
            // after installDependenciesInOrder, depInfo.isInstalled will be true if it succeeded
            if (depInfo.isInstalled) {
              toast({
                title: 'Dependency installed',
                description: `${depInfo.manifest.alias} installed successfully`,
              })
            }
          })
        } else {
          console.log('All dependencies already installed.')
        }
      }

      // Install main addon via store to keep global state in sync
      const success = await useAddonStore.getState().install(manifest, 'latest')
      didInstall = success

      if (!success) {
        toast({
          title: 'Error',
          description: `Failed to install ${manifest.alias}`,
          icon: AlertTriangleIcon,
        })
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err)

      if (message.includes('no release found')) {
        toast({
          title: 'Error',
          description: `No release found for ${manifest.name}`,
          icon: AlertTriangleIcon,
        })
      } else {
        toast({
          title: 'Error',
          description: `Failed to install ${manifest.alias}: ${message}`,
          icon: AlertTriangleIcon,
        })
      }

      console.error('Install error:', err)
    } finally {
      setIsProcessing(false)
    }

    if (didInstall) {
      toast({
        title: 'Addon installed',
        description: `${manifest.alias} was installed successfully.`,
      })
      setIsInstalled(true)
      onAddonInstalled?.()
      onOpenChange(false)
    }
  }

  const handleUninstall = async () => {
    if (isProcessing) return
    setIsProcessing(true)
    const [result, err] = await safeCall(LocalAddonService.UninstallAddon(manifest.name))
    setIsProcessing(false)
    if (err || !result) {
      toast({
        title: 'Error',
        description: `Failed to uninstall ${manifest.alias}`,
        icon: AlertTriangleIcon,
      })
      return
    }
    setIsInstalled(false)
    onAddonUninstalled?.()
    toast({
      title: 'Uninstalled',
      description: `${manifest.alias} uninstalled successfully`,
    })
    onOpenChange(false)
  }

  const getRelease = useCallback(async () => {
    const [r, err] = await safeCall<Release | null>(
      RemoteAddonService.GetLatestRelease(manifest.name)
    )
    if (err) {
      toast({
        title: 'Error',
        description: `Failed to fetch release information for ${manifest.name}`,
        icon: AlertTriangleIcon,
      })
      console.error('Fetch release error: ', err)
      setChangelog('Error loading change log')
      return
    }
    setRelease(r)
    if (r?.body) {
      setChangelog(r.body)
    } else {
      setChangelog('No change log was provided')
    }
  }, [manifest.name])

  const getReadme = useCallback(async () => {
    const [r, err] = await safeCall<Response>(
      fetch(
        `https://raw.githubusercontent.com/${manifest.repo}/refs/heads/${manifest.branch}/README.md`
      )
    )
    if (err) {
      console.error('Error fetching README: ', err)
      setReadme(manifest.description || 'Error loading description.')
      return
    }
    if (!r || !r.ok) {
      setReadme(manifest.description || 'No description provided.')
      return
    }

    const text = await r.text()
    setReadme(text)
  }, [manifest.repo, manifest.branch, manifest.description])

  const getMyRating = useCallback(async () => {
    if (!isAuthenticated) return
    apiClient
      .get(`/addon/${manifest.name}/my-rating`)
      .then(async response => {
        if (response.status === 200) {
          const r = await response.json()
          setRating(r.data.rating)
        }
      })
      .catch(e => {
        console.error('Failed to fetch rating: ', e)
      })
  }, [manifest.name, isAuthenticated])

  const getDependencies = useCallback(async () => {
    if (!manifest.dependencies || manifest.dependencies.length === 0) {
      setDependencies([])
      return
    }

    const [result, err] = await safeCall(resolveTransitiveDependencies(manifest))
    if (err || !result) {
      toast({
        title: 'Error',
        description: `Failed to resolve dependencies for ${manifest.name}`,
        icon: AlertTriangleIcon,
      })
      setDependencies([])
      return
    }

    if (result.errors.length > 0) {
      console.warn('Dependency resolution errors:', result.errors)
      // Show first error to user, log all errors
      toast({
        title: 'Dependency resolution warning',
        description: result.errors[0],
        icon: AlertTriangleIcon,
      })
    }

    setDependencies(result.dependencies)
    console.log(
      `Found ${dependencies.length} total dependencies (including transitive) for ${manifest.alias}`
    )

    // Log dependency tree for debugging
    if (dependencies.length > 0) {
      console.log('Dependency tree:')
      dependencies.forEach(dep => {
        console.log(
          `  ${'  '.repeat(dep.depth)}${dep.manifest.alias} (${dep.isInstalled ? 'installed' : 'not installed'})`
        )
      })
    }
  }, [manifest])

  const handleDependencyClick = (depManifest: AddonManifest) => {
    console.log('Clicked dependency:', depManifest.alias)
    onViewDependency(depManifest)
  }

  const handleRateAddon = async (newRating: number) => {
    await rateAddon(manifest.name, manifest.alias, newRating, rating, setRating)
  }

  return {
    release,
    readme,
    changelog,
    rating,
    dependencies,
    isInstalled,
    isProcessing,
    checkInstalledStatus,
    handleInstall,
    handleUninstall,
    getRelease,
    getReadme,
    getMyRating,
    getDependencies,
    handleDependencyClick,
    rateAddon: handleRateAddon,
  }
}
