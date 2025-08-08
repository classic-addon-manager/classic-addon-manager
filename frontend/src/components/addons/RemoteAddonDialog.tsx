import { Browser } from '@wailsio/runtime'
import {
  AlertTriangleIcon,
  BugIcon,
  CalendarDaysIcon,
  GithubIcon,
  PackageIcon,
  TagIcon,
  UserIcon,
} from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'

import { RemoteAddonReadme } from '@/components/shared/RemoteAddonReadme.tsx'
import { Badge } from '@/components/ui/badge.tsx'
import { Button } from '@/components/ui/button.tsx'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs.tsx'
import { toast } from '@/components/ui/toast.tsx'
import { apiClient } from '@/lib/api.ts'
import {
  type DependencyInfo,
  type DependencyResolutionResult,
  resolveTransitiveDependencies,
} from '@/lib/dependency-resolver.ts'
import { formatToLocalTime, safeCall } from '@/lib/utils.ts'
import type { AddonManifest } from '@/lib/wails'
import { type Release, RemoteAddonService } from '@/lib/wails'
import { useUserStore } from '@/stores/userStore.ts'

interface RemoteAddonDialogProps {
  manifest: AddonManifest
  open: boolean
  onOpenChange: (open: boolean) => void
  onViewDependency: (manifest: AddonManifest) => void
}

const Header = ({ manifest, release }: { manifest: AddonManifest; release: Release | null }) => {
  const [hasBanner, setHasBanner] = useState(true)
  const bannerUrl =
    'repo' in manifest && 'branch' in manifest
      ? `https://raw.githubusercontent.com/${manifest.repo}/${manifest.branch}/banner.png`
      : null

  const BannerImage = () => {
    if (bannerUrl && hasBanner) {
      return (
        <img
          className="absolute top-0 left-0 w-full h-full object-cover"
          src={bannerUrl}
          alt={`${manifest.alias} Banner`}
          onError={() => setHasBanner(false)}
        />
      )
    }
    return null
  }

  const ReleaseInformation = ({ release }: { release: Release | null }) => {
    if (!release) {
      return <Badge variant="outline">No Release</Badge>
    }

    // Possible todo, we need to handle case where there is a banner and colors may be hard to read
    return (
      <span className="inline-flex items-center gap-x-1.5">
        <Badge variant="secondary" className="inline-flex items-center gap-1">
          <TagIcon className="w-3 h-3" />
          {release.tag_name}
        </Badge>
        <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
          <CalendarDaysIcon className="w-3 h-3" />
          {formatToLocalTime(release.published_at)}
        </span>
      </span>
    )
  }

  if (!hasBanner) {
    return (
      <DialogHeader className="p-6 pb-4 shrink-0 border-b">
        <DialogTitle className="text-2xl font-semibold mb-1">{manifest.alias}</DialogTitle>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-muted-foreground">
          <span className="inline-flex items-center gap-1">
            <UserIcon className="w-3.5 h-3.5" />
            <span className="font-normal text-foreground/90">{manifest.author}</span>
          </span>
          <ReleaseInformation release={release} />
        </div>
        <div className="flex flex-wrap gap-2 text-sm">
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}`)}
          >
            <GithubIcon className="w-4 h-4" />
            View Code
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}/issues/new`)}
          >
            <BugIcon className="w-4 h-4" />
            Report Issue
          </Button>
        </div>
      </DialogHeader>
    )
  }

  return (
    <div className="relative w-full h-48 overflow-hidden shrink-0 rounded-t-lg border-b dark:border-white/10">
      <BannerImage />
      <div className="absolute inset-0 bg-gradient-to-t from-black/75 via-black/50 to-black/5"></div>
      <div className="absolute bottom-0 left-0 right-0 p-6 pb-4 pt-16 text-white bg-gradient-to-t from-black/60 to-transparent">
        <DialogTitle className="text-2xl font-semibold mb-1">{manifest.alias}</DialogTitle>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm mb-3 text-white/80">
          <span className="inline-flex items-center gap-1">
            <UserIcon className="w-3.5 h-3.5" />
            <span className="font-normal text-white/90">{manifest.author}</span>
          </span>
          <ReleaseInformation release={release} />
        </div>
        <div className="flex flex-wrap gap-x-2 gap-y-2 text-sm">
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1 border-white/50 text-white hover:bg-white/10"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}`)}
          >
            <GithubIcon className="w-4 h-4" />
            View Code
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="flex items-center gap-1 border-white/50 text-white hover:bg-white/10"
            onClick={() => Browser.OpenURL(`https://github.com/${manifest.repo}/issues/new`)}
          >
            <BugIcon className="w-4 h-4" />
            Report Issue
          </Button>
        </div>
      </div>
    </div>
  )
}

const Warning = ({ text }: { text: string | null | undefined }) => {
  if (!text) return null
  return (
    <div className="px-6 py-3 bg-yellow-100 dark:bg-yellow-900/30 border-y border-yellow-200 dark:border-yellow-800">
      <p className="text-sm text-yellow-800 dark:text-yellow-200 flex items-start gap-2">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="h-5 w-5 shrink-0 mt-0.5"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
          <line x1="12" y1="9" x2="12" y2="13" />
          <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
        {text}
      </p>
    </div>
  )
}

export const RemoteAddonDialog = ({
  manifest,
  open,
  onOpenChange,
  onViewDependency,
}: RemoteAddonDialogProps) => {
  const [release, setRelease] = useState<Release | null>(null)
  const [readme, setReadme] = useState<string>('Loading description...')
  const [changelog, setChangelog] = useState<string>('Loading changelog...')
  const [, setRating] = useState(0)
  const [dependencies, setDependencies] = useState<DependencyInfo[]>([])
  const { isAuthenticated } = useUserStore()
  const [currentTab, setCurrentTab] = useState<string>('description')

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

    const [result, err] = await safeCall<DependencyResolutionResult>(
      resolveTransitiveDependencies(manifest)
    )
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

  useEffect(() => {
    if (!open) {
      return
    }

    getMyRating().catch(e => {
      console.error('Failed to fetch rating: ', e)
    })
    getReadme().catch(e => {
      console.error('Failed to fetch readme: ', e)
    })
    getRelease().catch(e => {
      console.error('Failed to fetch release: ', e)
    })
    getDependencies().catch(e => {
      console.error('Failed to fetch dependencies: ', e)
    })
  }, [open, getMyRating, getReadme, getRelease, getDependencies])

  const tabs = () => {
    const baseTabs = [
      { value: 'description', label: 'Description' },
      { value: 'changelog', label: 'Changelog' },
    ]
    if (manifest.dependencies && manifest.dependencies.length > 0) {
      baseTabs.push({ value: 'dependencies', label: `Dependencies (${dependencies.length})` })
    }
    if (manifest.kofi) {
      baseTabs.push({ value: 'kofi', label: 'Support Author' })
    }
    return baseTabs
  }

  const handleDependencyClick = (depManifest: AddonManifest) => {
    console.log('Clicked dependency:', depManifest.alias)
    setCurrentTab('description')
    onViewDependency(depManifest)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[70%] lg:max-w-[60%] max-h-[90svh] flex flex-col p-0">
        <Header manifest={manifest} release={release} />
        <Warning text={manifest.warning} />

        <div className="flex-1 flex flex-col min-h-0 overflow-y-hidden px-6 pb-0">
          <Tabs
            className="w-full flex flex-col min-h-0"
            value={currentTab}
            onValueChange={setCurrentTab}
          >
            <TabsList className="inline-flex items-center bg-transparent justify-start gap-x-4 w-full mb-4">
              {tabs().map(tab => (
                <TabsTrigger
                  key={tab.value}
                  value={tab.value}
                  className="inline-flex items-center justify-center whitespace-nowrap px-3 py-2 text-sm font-medium transition-all focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50 border-transparent data-[state=active]:border-primary data-[state=active]:text-primary text-muted-foreground hover:text-primary"
                >
                  <span className="hover:text-foreground transition-colors">{tab.label}</span>
                </TabsTrigger>
              ))}
            </TabsList>

            <div className="flex-1 min-h-0 overflow-y-auto pb-2 pr-2 -mr-2">
              <TabsContent
                value="description"
                className="max-w-none"
                style={{ transform: 'translate(0)' }}
              >
                <RemoteAddonReadme readme={readme} />
              </TabsContent>

              <TabsContent value="changelog" className="text-sm">
                {release ? (
                  <>
                    <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground mb-4">
                      <CalendarDaysIcon className="w-3.5 h-3.5" />
                      <span>Released on {formatToLocalTime(release.published_at)}</span>
                    </div>
                    <div className="border rounded-lg p-4 bg-card">
                      <div className="prose max-w-none text-sm text-foreground dark:text-foreground/90">
                        <RemoteAddonReadme readme={changelog} />
                      </div>
                    </div>
                  </>
                ) : (
                  <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
                    <PackageIcon className="w-12 h-12 mb-4 opacity-50" />
                    <p className="font-medium">{changelog}</p>
                    {changelog === 'No change log was provided' ? (
                      <p className="text-xs mt-1">The author hasn't provided release notes.</p>
                    ) : (
                      <></>
                    )}
                    {changelog === 'Error loading change log' ? (
                      <p className="text-xs mt-0 text-destructive">
                        Could not fetch release details.
                      </p>
                    ) : (
                      <></>
                    )}
                  </div>
                )}
              </TabsContent>

              {dependencies.length > 0 && (
                <TabsContent value="dependencies">
                  <p className="mb-4 text-sm text-muted-foreground">
                    This addon requires the following dependencies (including transitive
                    dependencies):
                  </p>
                  <div className="space-y-3">
                    {dependencies.map(d => (
                      <button
                        key={d.manifest.name}
                        className="flex flex-col w-full p-4 border rounded-lg bg-background hover:bg-muted/50 transition-colors text-left focus:outline-none focus:ring-0 focus:ring-offset-0"
                        onClick={() => handleDependencyClick(d.manifest)}
                        aria-label={`View details for ${d.manifest.alias}`}
                      >
                        <div className="flex justify-between items-start mb-1 gap-2">
                          <span className="font-medium text-base flex-1 break-words">
                            {d.manifest.alias}
                          </span>
                          {d.isInstalled ? (
                            <Badge variant="secondary" className="text-xs whitespace-nowrap">
                              Installed
                            </Badge>
                          ) : (
                            <Badge variant="outline" className="text-xs whitespace-nowrap">
                              Not Installed
                            </Badge>
                          )}
                        </div>
                        <p className="text-sm text-muted-foreground mb-2">by {d.manifest.author}</p>
                        <p className="text-sm text-foreground/80 line-clamp-2">
                          {d.manifest.description || 'No description available.'}
                        </p>
                      </button>
                    ))}
                  </div>
                </TabsContent>
              )}
            </div>
          </Tabs>
        </div>
      </DialogContent>
    </Dialog>
  )
}
