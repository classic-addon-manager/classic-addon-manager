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
import { useTitleBarSlot } from '@/hooks/useTitleBarSlot'
import { safeCall } from '@/lib/utils'

const AddonsToolbar = () => {
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

  const toolbarContent = (
    <div className="flex w-full min-w-0 items-center gap-3">
      <div className="no-drag relative min-w-0 flex-1">
        <SearchIcon className="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 transform" />
        <Input
          disabled={!isReady}
          type="search"
          placeholder="Search addons..."
          className="w-full pl-10 pr-4 shadow-none transition-colors focus-visible:ring-1"
          onChange={event => debouncedSetSearch(event.target.value)}
        />
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <div className="no-drag flex items-center gap-2">
          <Select onValueChange={setSelectedTag} disabled={!isReady} value={selectedTag}>
            <SelectTrigger className="h-8 w-[140px]">{selectedTag}</SelectTrigger>
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
            size="sm"
            disabled={!isReady || isRefreshing}
            className="flex h-8 min-w-[120px] w-[120px] items-center justify-center gap-2 transition-all duration-200 hover:shadow-md"
            onClick={onRefresh}
          >
            <RefreshCw className={clsx('h-4 w-4', isRefreshing && 'animate-spin')} />
            {isRefreshing ? 'Refreshing' : 'Refresh'}
          </Button>
        </div>
      </div>
    </div>
  )

  useTitleBarSlot(toolbarContent)

  return null
}

export const Addons = () => {
  const loadAddons = useSetAtom(loadAddonsAtom)
  const [selectedManifest, setSelectedManifest] = useAtom(selectedManifestAtom)
  const [isDialogOpen, setDialogOpen] = useAtom(isManifestDialogOpenAtom)

  useEffect(() => {
    loadAddons()
  }, [])

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AddonsToolbar />
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

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="container mx-auto px-4">
          <AddonList />
        </div>
      </div>
    </div>
  )
}
