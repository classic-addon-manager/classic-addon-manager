import clsx from 'clsx'
import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import { AlertTriangleIcon, RefreshCw, SearchIcon } from 'lucide-react'
import { useEffect } from 'react'
import { useDebouncedCallback } from 'use-debounce'

import { AddonList } from '@/components/addons/AddonList'
import {
  isAddonsReadyAtom,
  isManifestDialogOpenAtom,
  isRefreshingAtom,
  loadAddonsAtom,
  searchQueryAtom,
  selectedManifestAtom,
  selectedTagAtom,
  tagsAtom,
} from '@/components/addons/atoms'
import { RemoteAddonDialog } from '@/components/addons/RemoteAddonDialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select'
import { toast } from '@/components/ui/toast'
import { safeCall } from '@/lib/utils'

const Header = () => {
  const loadAddons = useSetAtom(loadAddonsAtom)
  const isReady = useAtomValue(isAddonsReadyAtom)
  const setSearchQuery = useSetAtom(searchQueryAtom)
  const [selectedTag, setSelectedTag] = useAtom(selectedTagAtom)
  const tags = useAtomValue(tagsAtom)
  const [isRefreshing, setIsRefreshing] = useAtom(isRefreshingAtom)

  const debouncedSetSearch = useDebouncedCallback((value: string) => {
    setSearchQuery(value)
  }, 300)

  const onRefresh = async () => {
    if (isRefreshing) return
    setIsRefreshing(true)

    const startTime = Date.now()
    const [, err] = await safeCall(loadAddons())
    if (err) {
      console.error('Failed to refresh addons', err)
      toast({
        title: 'Error',
        description: `Failed to refresh addons: ${err.message.substring(0, 100)}`,
        icon: AlertTriangleIcon,
      })
    }

    const elapsedTime = Date.now() - startTime
    if (elapsedTime < 500) {
      await new Promise(resolve => setTimeout(resolve, 500 - elapsedTime))
    }

    setIsRefreshing(false)
    toast({
      title: 'Completed',
      description: `Refreshed addons in ${elapsedTime}ms`,
      icon: RefreshCw,
    })
  }

  return (
    <header className="bg-background/95 backdrop-blur-sm supports-[backdrop-filter]:bg-background/60 border-b">
      <div className="container flex h-16 items-center gap-4 px-4">
        <div className="relative flex-1">
          <SearchIcon className="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 transform" />
          <Input
            disabled={!isReady}
            type="search"
            placeholder="Search addons..."
            className="bg-background w-full pl-10 shadow-none transition-colors focus-visible:bg-background/80"
            onChange={event => debouncedSetSearch(event.target.value)}
          />
        </div>
        <div className="flex items-center gap-3 shrink-0">
          <Select onValueChange={setSelectedTag} disabled={!isReady}>
            <SelectTrigger className="w-[140px] bg-background/80">{selectedTag}</SelectTrigger>
            <SelectContent>
              {tags.map(tag => (
                <SelectItem key={tag} value={tag}>
                  {tag}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Button
            variant="outline"
            disabled={!isReady || isRefreshing}
            className="min-w-[120px] w-[120px] flex items-center justify-center gap-2 transition-all duration-200 hover:shadow-md"
            onClick={onRefresh}
          >
            <RefreshCw className={clsx('h-4 w-4', isRefreshing && 'animate-spin')} />
            {isRefreshing ? 'Refreshing' : 'Refresh'}
          </Button>
        </div>
      </div>
    </header>
  )
}

export const Addons = () => {
  const loadAddons = useSetAtom(loadAddonsAtom)
  const [selectedManifest, setSelectedManifest] = useAtom(selectedManifestAtom)
  const [isDialogOpen, setDialogOpen] = useAtom(isManifestDialogOpenAtom)

  useEffect(() => {
    loadAddons()
  }, [])

  return (
    <div className="flex flex-col h-screen">
      <Header />
      {selectedManifest && (
        <RemoteAddonDialog
          manifest={selectedManifest}
          open={isDialogOpen}
          onOpenChange={open => {
            setDialogOpen(open)
            if (!open) {
              setTimeout(() => setSelectedManifest(null), 200)
            }
          }}
          onViewDependency={manifest => {
            setSelectedManifest(manifest)
            setDialogOpen(true)
          }}
          onAddonInstalled={loadAddons}
          onAddonUninstalled={loadAddons}
        />
      )}

      <div className="flex-1 overflow-auto">
        <div className="container px-4">
          <AddonList />
        </div>
      </div>
    </div>
  )
}
