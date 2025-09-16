import { useSetAtom } from 'jotai'
import { AlertCircle, CalendarDays, CheckCircle2Icon, LoaderCircle, TagIcon } from 'lucide-react'
import { Suspense, use, useEffect, useState } from 'react'
import { toast } from 'sonner'

import { versionSelectAtom } from '@/components/dashboard/atoms.ts'
import { Button } from '@/components/ui/button.tsx'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { cn } from '@/lib/utils'
import type { Addon, Release } from '@/lib/wails'

interface ApiResponse {
  status: boolean
  message: string
  data: Release[]
}

interface Props {
  addon: Addon
}

const LoadingState = () => (
  <div className="flex items-center justify-center p-8">
    <LoaderCircle className="h-8 w-8 animate-spin text-muted-foreground" />
    <span className="ml-3 text-muted-foreground">Loading versions...</span>
  </div>
)

const EmptyState = () => (
  <div className="flex items-center justify-center gap-3 p-8 text-center text-muted-foreground">
    <AlertCircle size={18} />
    <span>No other versions found for this addon.</span>
  </div>
)

const ReleasesContent = ({
  addon,
  releases,
  selectedVersion,
  onVersionChange,
}: {
  addon: Addon
  releases: Release[]
  selectedVersion: string | null
  onVersionChange: (version: string) => void
}) => {
  const isCurrentVersion = (version: string) => version === addon.version

  const selectedRelease = releases.find(r => r.tag_name === selectedVersion)
  const triggerContent = selectedVersion || 'Select a version...'

  return (
    <div className="py-2">
      <label htmlFor="version-select" className="text-sm font-medium mb-2 block">
        Version
      </label>
      <Select value={selectedVersion || ''} onValueChange={onVersionChange}>
        <SelectTrigger className="w-full bg-card">
          <div className="flex items-center gap-2">
            {selectedVersion && <TagIcon size={14} className="text-primary" />}
            <SelectValue placeholder={triggerContent} />
          </div>
        </SelectTrigger>
        <SelectContent className="max-h-[300px] overflow-y-auto" align="start" sideOffset={4}>
          <SelectGroup>
            <SelectLabel className="font-medium px-2 py-1.5 text-xs uppercase">
              Available Versions
            </SelectLabel>
            {releases.map(release => (
              <SelectItem
                key={release.tag_name}
                value={release.tag_name}
                className={cn(
                  'py-2.5 pl-8 pr-2 w-full',
                  isCurrentVersion(release.tag_name) && 'bg-muted/50'
                )}
              >
                <div className="flex items-center w-full">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{release.tag_name}</span>
                    {isCurrentVersion(release.tag_name) && (
                      <span className="bg-primary/15 text-primary text-[10px] px-1.5 py-0.5 rounded-full uppercase font-medium">
                        Current
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-1 text-xs text-muted-foreground absolute right-8">
                    <CalendarDays size={12} />
                    <span>
                      {new Date(release.published_at).toLocaleDateString(undefined, {
                        day: '2-digit',
                        month: 'short',
                        year: 'numeric',
                      })}
                    </span>
                  </div>
                </div>
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>

      {selectedRelease?.body && (
        <div className="mt-4 text-sm">
          <div className="font-medium text-sm mb-1">Release Notes:</div>
          <div className="bg-muted p-3 rounded-md text-xs max-h-[100px] overflow-y-auto whitespace-pre-line">
            {selectedRelease.body}
          </div>
        </div>
      )}
    </div>
  )
}

const releaseCache = new Map<string, Promise<Release[]>>()

const fetchReleases = async (addon: Addon): Promise<Release[]> => {
  if (!addon) return []

  const cacheKey = addon.name
  if (releaseCache.has(cacheKey)) {
    return releaseCache.get(cacheKey)!
  }

  const promise = (async () => {
    try {
      const response = await fetch(`https://aac.gaijin.dev/addon/${addon.name}/releases`)
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`)
      }
      const result: ApiResponse = await response.json()

      if (result.status && result.data) {
        // Filter out duplicates based on tag_name
        const uniqueReleasesMap = new Map<string, Release>()
        for (const release of result.data) {
          if (!uniqueReleasesMap.has(release.tag_name)) {
            uniqueReleasesMap.set(release.tag_name, release)
          }
        }

        return Array.from(uniqueReleasesMap.values())
      } else {
        throw new Error(result.message || 'Failed to fetch releases: Invalid API response')
      }
    } catch (e: unknown) {
      console.error(`Failed to fetch releases for ${addon.name}:`, e)
      const errorMessage = e instanceof Error ? e.message : 'An unknown error occurred.'
      toast.error(`Failed to load versions: ${errorMessage}`)
      throw e
    }
  })()

  releaseCache.set(cacheKey, promise)
  return promise
}

interface ReleasesProps {
  addon: Addon
  selectedVersion: string | null
  onVersionChange: (version: string) => void
  onReleasesLoaded: (releases: Release[]) => void
}

const Releases = ({ addon, selectedVersion, onVersionChange, onReleasesLoaded }: ReleasesProps) => {
  const releasesPromise = fetchReleases(addon)
  const releases = use(releasesPromise)

  useEffect(() => {
    if (releases.length > 0) {
      onReleasesLoaded(releases)
    }
  }, [releases, onReleasesLoaded])

  if (releases.length === 0) {
    return <EmptyState />
  }

  return (
    <ReleasesContent
      addon={addon}
      releases={releases}
      selectedVersion={selectedVersion}
      onVersionChange={onVersionChange}
    />
  )
}

interface VersionSelectSuspenseProps {
  addon: Addon
  selectedVersion: string | null
  onReleasesLoaded: (releases: Release[]) => void
  onVersionChange: (version: string) => void
}

const VersionSelectSuspense = ({
  addon,
  selectedVersion,
  onReleasesLoaded,
  onVersionChange,
}: VersionSelectSuspenseProps) => {
  return (
    <Suspense fallback={<LoadingState />}>
      <Releases
        addon={addon}
        selectedVersion={selectedVersion}
        onVersionChange={onVersionChange}
        onReleasesLoaded={onReleasesLoaded}
      />
    </Suspense>
  )
}

export const LocalAddonVersionSelectDialog = ({ addon }: Props) => {
  const setAddon = useSetAtom(versionSelectAtom)
  const [selectedVersion, setSelectedVersion] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [releases, setReleases] = useState<Release[]>([])

  const handleInstall = async () => {
    if (!selectedVersion) return

    setIsLoading(true)
    setError(null)
    try {
      // TODO: Implement install logic
      console.log(`Installing version ${selectedVersion} for ${addon.name}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to install')
    } finally {
      setIsLoading(false)
    }
  }

  const setOpen = (open: boolean) => {
    if (!open) {
      setAddon(null)
    }
  }

  return (
    <Dialog onOpenChange={setOpen} open={addon !== null}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader className="pb-4 border-b">
          <DialogTitle className="flex items-center gap-2 text-xl">
            <TagIcon size={18} className="text-primary" />
            Install Version
          </DialogTitle>
          <DialogDescription className="text-sm text-muted-foreground">
            Select a version of <span className="font-medium text-foreground">{addon.alias} </span>
            to install.
          </DialogDescription>
          <div className="mt-1 flex items-center gap-1.5 text-xs">
            <span className="inline-flex items-center bg-muted px-2 py-0.5 rounded text-muted-foreground">
              Current: <span className="font-medium ml-1">{addon.version}</span>
            </span>
          </div>
        </DialogHeader>
        <VersionSelectSuspense
          addon={addon}
          selectedVersion={selectedVersion}
          onReleasesLoaded={setReleases}
          onVersionChange={setSelectedVersion}
        />

        {error && (
          <div className="flex items-center gap-2 p-3 rounded-md bg-destructive/10 border border-destructive/20 text-destructive text-sm">
            <AlertCircle size={16} />
            <span>{error}</span>
          </div>
        )}

        <DialogFooter className="border-t pt-4 gap-2">
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleInstall}
            disabled={isLoading || !!error || !selectedVersion || releases.length === 0}
            className="gap-2"
          >
            {selectedVersion ? (
              <>
                <CheckCircle2Icon size={16} />
                Install {selectedVersion}
              </>
            ) : (
              'Install Selected Version'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
