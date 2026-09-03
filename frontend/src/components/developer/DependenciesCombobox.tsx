import { Check, ChevronsUpDown, X } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Command,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { cn } from '@/lib/utils'
import type { AddonManifest } from '@/lib/wails'
import { RemoteAddonService } from '@/lib/wails'

type CatalogStatus = 'loading' | 'error' | 'success'

interface DependenciesComboboxProps {
  selected: string[]
  onChange: (names: string[]) => void
}

export const DependenciesCombobox = ({ selected, onChange }: DependenciesComboboxProps) => {
  const [open, setOpen] = useState(false)
  const [manifests, setManifests] = useState<AddonManifest[]>([])
  const [status, setStatus] = useState<CatalogStatus>('loading')

  const loadCatalog = useCallback(async () => {
    setStatus('loading')
    try {
      const result = await RemoteAddonService.GetAddonManifest()
      setManifests(result ?? [])
      setStatus('success')
    } catch {
      setStatus('error')
    }
  }, [])

  useEffect(() => {
    void loadCatalog()
  }, [loadCatalog])

  const aliasFor = (name: string) =>
    manifests.find(manifest => manifest.name === name)?.alias ?? name

  const toggle = (name: string) => {
    if (selected.includes(name)) {
      onChange(selected.filter(item => item !== name))
      return
    }
    onChange([...selected, name])
  }

  return (
    <div className="space-y-2">
      {selected.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {selected.map(name => (
            <Badge key={name} variant="default" className="gap-1 rounded-full pr-1">
              {aliasFor(name)}
              <button
                type="button"
                onClick={() => onChange(selected.filter(item => item !== name))}
                className="rounded-full p-0.5 hover:bg-primary-foreground/20"
                aria-label={`Remove ${aliasFor(name)}`}
              >
                <X className="size-3" />
              </button>
            </Badge>
          ))}
        </div>
      )}

      {status === 'loading' && (
        <div className="flex h-9 items-center rounded-md border border-input px-3 text-sm text-muted-foreground">
          Loading addons…
        </div>
      )}

      {status === 'error' && (
        <div className="flex h-9 items-center justify-between gap-2 rounded-md border border-input px-3">
          <span className="text-sm text-muted-foreground">Couldn't load addons.</span>
          <Button type="button" variant="outline" size="sm" onClick={() => void loadCatalog()}>
            Retry
          </Button>
        </div>
      )}

      {status === 'success' && manifests.length === 0 && (
        <div className="flex h-9 items-center rounded-md border border-input px-3 text-sm text-muted-foreground">
          No addons in the catalog.
        </div>
      )}

      {status === 'success' && manifests.length > 0 && (
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="outline"
              role="combobox"
              aria-expanded={open}
              className="w-full justify-between font-normal"
            >
              <span className="truncate text-muted-foreground">Select addons</span>
              <ChevronsUpDown className="size-4 shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>
          <PopoverContent
            className="w-(--radix-popover-trigger-width) p-0"
            align="start"
            side="top"
            avoidCollisions={false}
          >
            <Command>
              <CommandInput />
              <CommandList>
                <CommandGroup>
                  {manifests.map(manifest => {
                    const isSelected = selected.includes(manifest.name)
                    return (
                      <CommandItem
                        key={manifest.name}
                        value={`${manifest.alias} ${manifest.name}`}
                        onSelect={() => toggle(manifest.name)}
                      >
                        <Check
                          className={cn(
                            'size-4',
                            isSelected ? 'text-primary opacity-100' : 'opacity-0'
                          )}
                        />
                        {manifest.alias}
                      </CommandItem>
                    )
                  })}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
      )}
    </div>
  )
}
