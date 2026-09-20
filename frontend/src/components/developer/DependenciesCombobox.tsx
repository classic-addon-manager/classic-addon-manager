import { Check, ChevronsUpDown, X } from 'lucide-react'
import { useState } from 'react'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Command,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command'
import { Popover, PopoverAnchor, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useAddonCatalog } from '@/lib/catalog'
import { cn } from '@/lib/utils'

interface DependenciesComboboxProps {
  selected: string[]
  onChange: (names: string[]) => void
  invalid?: boolean
  disabled?: boolean
}

export const DependenciesCombobox = ({
  selected,
  onChange,
  invalid = false,
  disabled = false,
}: DependenciesComboboxProps) => {
  const [open, setOpen] = useState(false)
  const { data: manifests = [], isPending, isError, refetch } = useAddonCatalog()

  if (disabled && open) {
    setOpen(false)
  }

  const aliasFor = (name: string) =>
    manifests.find(manifest => manifest.name === name)?.alias ?? name

  const toggle = (name: string) => {
    if (selected.includes(name)) {
      onChange(selected.filter(item => item !== name))
      return
    }
    onChange([...selected, name])
  }

  const statusRow =
    isPending ? (
      <div className="flex h-9 items-center px-3 text-sm text-muted-foreground">
        Loading addons…
      </div>
    ) : isError ? (
      <div className="flex h-9 items-center justify-between gap-2 px-3">
        <span className="text-sm text-muted-foreground">Couldn't load addons.</span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={disabled}
          onClick={() => void refetch()}
        >
          Retry
        </Button>
      </div>
    ) : manifests.length === 0 ? (
      <div className="flex h-9 items-center px-3 text-sm text-muted-foreground">
        No addons available.
      </div>
    ) : null

  const pillFor = (name: string) => (
    <Badge key={name} variant="default" className="max-w-full gap-1 rounded-full pr-1">
      <span className="truncate">{aliasFor(name)}</span>
      <button
        type="button"
        disabled={disabled}
        onClick={() => onChange(selected.filter(item => item !== name))}
        className="pointer-events-auto rounded-full p-0.5 hover:bg-primary-foreground/20 disabled:pointer-events-none"
        aria-label={`Remove ${aliasFor(name)}`}
      >
        <X className="size-3" />
      </button>
    </Badge>
  )

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverAnchor asChild>
        <div
          className={cn(
            'dark:bg-input/30 border-input relative flex min-h-9 w-full flex-col rounded-md border bg-transparent shadow-xs transition-[color,box-shadow]',
            'focus-within:border-ring focus-within:ring-ring/50 focus-within:ring-[3px]',
            invalid &&
              'border-destructive focus-within:border-destructive focus-within:ring-destructive/20 dark:focus-within:ring-destructive/40',
            disabled && 'pointer-events-none cursor-not-allowed opacity-50'
          )}
        >
          {statusRow ?? (
            <div className="relative grid min-h-9 w-full grid-cols-[minmax(0,1fr)_2.25rem]">
              <PopoverTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={disabled}
                  aria-expanded={open}
                  aria-invalid={invalid || undefined}
                  aria-label={`Choose dependencies, ${selected.length} selected`}
                  className="absolute inset-0 z-0 h-auto w-auto justify-start rounded-md px-3"
                >
                  {selected.length === 0 && (
                    <span className="text-sm font-normal text-muted-foreground">Select addons</span>
                  )}
                </Button>
              </PopoverTrigger>
              {selected.length > 0 && (
                <div className="pointer-events-none relative z-10 col-start-1 row-start-1 flex min-w-0 flex-wrap items-center gap-1.5 py-2 pl-3">
                  {selected.map(name => pillFor(name))}
                </div>
              )}
              <div className="pointer-events-none relative z-10 col-start-2 row-start-1 flex h-9 items-center justify-center self-start">
                <ChevronsUpDown aria-hidden className="size-4 opacity-50" />
              </div>
            </div>
          )}

          {statusRow && selected.length > 0 && (
            <div className="flex flex-wrap gap-1.5 px-3 pb-2">
              {selected.map(name => pillFor(name))}
            </div>
          )}
        </div>
      </PopoverAnchor>
      <PopoverContent
        className="w-(--radix-popover-trigger-width) p-0"
        align="start"
        side="top"
        avoidCollisions={false}
      >
        <Command label="Addons">
          <CommandInput placeholder="Search addons…" />
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
  )
}
