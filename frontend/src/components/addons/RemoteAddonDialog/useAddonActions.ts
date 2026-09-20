import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { AlertTriangleIcon } from 'lucide-react'

import { toast } from '@/components/ui/toast.tsx'
import { notifyDependencyResult } from '@/lib/notifyDependencyResult'
import { useAddonReadme } from '@/lib/repo'
import { safeCall } from '@/lib/utils.ts'
import type { AddonManifest } from '@/lib/wails'
import { LocalAddonService, RemoteAddonService } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore.ts'

export const addonActionMutationKey = (name: string) => ['addon-action', name] as const

interface UseAddonActionsProps {
  manifest: AddonManifest
  open: boolean
  onViewDependency: (manifest: AddonManifest) => void
  onOpenChange: (open: boolean) => void
  onAddonInstalled?: () => void
  onAddonUninstalled?: () => void
}

export const useAddonActions = ({
  manifest,
  open,
  onViewDependency,
  onOpenChange,
  onAddonInstalled,
  onAddonUninstalled,
}: UseAddonActionsProps) => {
  const queryClient = useQueryClient()
  const mutationKey = addonActionMutationKey(manifest.name)
  const installedQueryKey = ['addon-installed', manifest.name] as const

  const installedQuery = useQuery({
    queryKey: installedQueryKey,
    enabled: open,
    queryFn: async () => {
      const [installed] = await safeCall(LocalAddonService.IsInstalled(manifest.name))
      return !!installed
    },
  })

  const releaseQuery = useQuery({
    queryKey: ['addon-release', manifest.name],
    enabled: open,
    retry: false,
    queryFn: async () => {
      const [r, err] = await safeCall(RemoteAddonService.GetLatestRelease(manifest.name))
      if (err) {
        toast({
          title: 'Error',
          description: `Failed to fetch release information for ${manifest.name}`,
          icon: AlertTriangleIcon,
        })
        console.error('Fetch release error: ', err)
        throw err
      }
      return r
    },
  })

  const readmeQuery = useAddonReadme(manifest.repo, manifest.branch, open)

  const dependenciesQuery = useQuery({
    queryKey: ['addon-deps', manifest.name],
    enabled: open && manifest.dependencies.length > 0,
    retry: false,
    queryFn: async () => {
      const [result, err] = await safeCall(RemoteAddonService.ResolveDependencies(manifest))
      if (err || !result) {
        toast({
          title: 'Error',
          description: `Failed to resolve dependencies for ${manifest.name}`,
          icon: AlertTriangleIcon,
        })
        return []
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

      console.log(
        `Found ${result.dependencies.length} total dependencies (including transitive) for ${manifest.alias}`
      )

      // Log dependency tree for debugging
      if (result.dependencies.length > 0) {
        console.log('Dependency tree:')
        result.dependencies.forEach(dep => {
          console.log(
            `  ${'  '.repeat(dep.depth)}${dep.manifest.alias} (${dep.isInstalled ? 'installed' : 'not installed'})`
          )
        })
      }

      return result.dependencies
    },
  })

  const readme =
    (readmeQuery.isError ? null : readmeQuery.data) ||
    manifest.description ||
    'No description provided.'
  const release = releaseQuery.data ?? null
  const isInstalled = installedQuery.data ?? false
  const changelog = releaseQuery.isPending
    ? ''
    : releaseQuery.isError
      ? 'Error loading change log'
      : release?.body || 'No change log was provided'

  const installMutation = useMutation({
    mutationKey,
    mutationFn: async () => {
      const [installResult, installErr] = await safeCall(
        RemoteAddonService.InstallAddonWithDependencies(manifest, 'latest')
      )
      if (installErr || !installResult) {
        throw installErr ?? new Error('Install failed')
      }
      return installResult
    },
    onSuccess: async installResult => {
      if (!notifyDependencyResult(installResult, `Failed to install ${manifest.alias}`)) {
        await useAddonStore.getState().refreshAfterAddonChange()
        return
      }
      await useAddonStore.getState().refreshAfterAddonChange()
      toast({
        title: 'Addon installed',
        description: `${manifest.alias} was installed successfully.`,
      })
      await queryClient.invalidateQueries({ queryKey: installedQueryKey })
      onAddonInstalled?.()
      onOpenChange(false)
    },
    onError: (err: unknown) => {
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
    },
  })

  const uninstallMutation = useMutation({
    mutationKey,
    mutationFn: async () => {
      const [result, err] = await safeCall(LocalAddonService.UninstallAddon(manifest.name))
      if (err || !result) throw err ?? new Error('Uninstall failed')
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: installedQueryKey })
      onAddonUninstalled?.()
      toast({
        title: 'Uninstalled',
        description: `${manifest.alias} uninstalled successfully`,
      })
      onOpenChange(false)
    },
    onError: () => {
      toast({
        title: 'Error',
        description: `Failed to uninstall ${manifest.alias}`,
        icon: AlertTriangleIcon,
      })
    },
  })

  const handleInstall = () => {
    if (installMutation.isPending || uninstallMutation.isPending) return

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

    installMutation.mutate()
  }

  const handleUninstall = () => {
    if (installMutation.isPending || uninstallMutation.isPending) return
    uninstallMutation.mutate()
  }


  const handleDependencyClick = (depManifest: AddonManifest) => {
    console.log('Clicked dependency:', depManifest.alias)
    onViewDependency(depManifest)
  }

  return {
    release,
    readme,
    changelog,
    dependencies: dependenciesQuery.data ?? [],
    isInstalled,
    isLoadingRelease: open && releaseQuery.isPending,
    isLoadingReadme: readmeQuery.isLoading,
    handleInstall,
    handleUninstall,
    handleDependencyClick,
  }
}
