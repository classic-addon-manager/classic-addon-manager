import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import {
  AlertTriangleIcon,
  LayoutGrid,
  List,
  LoaderCircle,
  RefreshCw,
  SearchIcon,
} from 'lucide-react'
import { useDebouncedCallback } from 'use-debounce'

import {
  isAddonsReadyAtom,
  isRefreshingAtom,
  loadAddonsAtom,
  searchQueryAtom,
  selectedTagAtom,
  tagsAtom,
} from '@/components/addons/atoms'
import type { AddonViewMode } from '@/components/addons/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select'
import { toast } from '@/components/ui/toast'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { useTitleBarSlot } from '@/hooks/useTitleBarSlot'
import { safeCall } from '@/lib/utils'
import { usePreferencesStore } from '@/stores/preferencesStore'

export const AddonsToolbar = () => {
  const loadAddons = useSetAtom(loadAddonsAtom)
  const isReady = useAtomValue(isAddonsReadyAtom)
  const setSearchQuery = useSetAtom(searchQueryAtom)
  const [selectedTag, setSelectedTag] = useAtom(selectedTagAtom)
  const tags = useAtomValue(tagsAtom)
  const [isRefreshing, setIsRefreshing] = useAtom(isRefreshingAtom)
  const viewMode = usePreferencesStore(s => s.addonViewMode)
  const setAddonViewMode = usePreferencesStore(s => s.setAddonViewMode)

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

          <ToggleGroup
            type="single"
            variant="outline"
            size="sm"
            value={viewMode}
            onValueChange={value => value && setAddonViewMode(value as AddonViewMode)}
            disabled={!isReady}
          >
            <ToggleGroupItem value="list" aria-label="List view">
              <List className="h-4 w-4" />
            </ToggleGroupItem>
            <ToggleGroupItem value="grid" aria-label="Grid view">
              <LayoutGrid className="h-4 w-4" />
            </ToggleGroupItem>
          </ToggleGroup>

          <Button
            variant="outline"
            size="sm"
            disabled={!isReady || isRefreshing}
            className="h-8 transition-all duration-200 hover:shadow-md"
            onClick={onRefresh}
          >
            {isRefreshing ? (
              <>
                <LoaderCircle className="mr-1.5 size-3.5 animate-spin" />
                Refresh
              </>
            ) : (
              <>
                <RefreshCw className="mr-1.5 size-3.5" />
                Refresh
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  )

  useTitleBarSlot(toolbarContent)

  return null
}
