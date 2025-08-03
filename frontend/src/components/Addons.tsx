import { useAtom, useAtomValue, useSetAtom } from 'jotai'
import { SearchIcon } from 'lucide-react'
import { useEffect } from 'react'
import { useDebouncedCallback } from 'use-debounce'

import {
  filteredAddonsAtom,
  // addonsAtom,
  isAddonsReadyAtom,
  loadAddonsAtom,
  searchQueryAtom,
  selectedTagAtom,
  tagsAtom,
} from '@/components/addons/atoms.ts'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select'

const Header = () => {
  const isReady = useAtomValue(isAddonsReadyAtom)
  const setSearchQuery = useSetAtom(searchQueryAtom)
  const [selectedTag, setSelectedTag] = useAtom(selectedTagAtom)
  const tags = useAtomValue(tagsAtom)

  const debouncedSetSearch = useDebouncedCallback((value: string) => {
    setSearchQuery(value)
  }, 300)

  useEffect(() => {
    console.log(selectedTag)
  }, [selectedTag])

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
        </div>
      </div>
    </header>
  )
}

export const Addons = () => {
  const loadAddons = useSetAtom(loadAddonsAtom)
  const filteredAddons = useAtomValue(filteredAddonsAtom)

  useEffect(() => {
    loadAddons()
  }, [])

  return (
    <div className="flex flex-col h-screen">
      <Header />
      {filteredAddons.map(addon => (
        <div key={addon.manifest.name} className="flex items-center justify-between p-4 border-b">
          {addon.manifest.alias}
        </div>
      ))}
    </div>
  )
}
