import { useQuery } from '@tanstack/react-query'
import { AnimatePresence, motion } from 'framer-motion'
import { useAtomValue, useSetAtom } from 'jotai'
import {
  AlertCircle,
  AlertTriangle,
  ArrowRight,
  ArrowUp,
  Check,
  Download,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
  SearchIcon,
  TagIcon,
} from 'lucide-react'
import { type ComponentType, type KeyboardEvent, useEffect, useRef, useState } from 'react'

import { versionSelectAtom } from '@/components/dashboard/atoms.ts'
import { catalogIconMapAtom } from '@/components/dashboard/iconMap'
import { AddonIcon } from '@/components/shared/AddonIcon'
import { Readme } from '@/components/shared/Readme'
import { Button } from '@/components/ui/button.tsx'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast.tsx'
import { apiClient } from '@/lib/api.ts'
import { dependencyFailure, notifyDependencyProgress } from '@/lib/notifyDependencyResult'
import { repoGetManifest } from '@/lib/repo'
import { cn, formatToLocalDate, safeCall } from '@/lib/utils'
import type { Addon, Release } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

interface ApiResponse {
  status: boolean
  message: string
  data: Release[]
}

interface Props {
  addon: Addon
}

/** How long the success state stays up before the dialog closes itself. */
const AUTO_CLOSE_MS = 1800

const DAY_MS = 86_400_000

const eyebrowClass = 'text-[10px] leading-4 font-semibold tracking-[0.14em] uppercase'

const kbdClass =
  'inline-grid h-5 min-w-5 place-items-center rounded-md border border-border bg-card px-1 font-mono text-[10px] leading-none text-muted-foreground'

/** Where the selected release sits relative to the installed one. */
type Direction = 'upgrade' | 'rollback' | 'reinstall' | 'install'

const directions: Record<
  Direction,
  {
    label: string
    action: string
    running: string
    done: string
    icon: ComponentType<{ className?: string }>
    tone: string
  }
> = {
  upgrade: {
    label: 'Upgrade',
    action: 'Upgrade to',
    running: 'Upgrading…',
    done: 'upgraded',
    icon: ArrowUp,
    tone: 'text-emerald-500',
  },
  rollback: {
    label: 'Rollback',
    action: 'Roll back to',
    running: 'Rolling back…',
    done: 'rolled back',
    icon: RotateCcw,
    tone: 'text-primary',
  },
  reinstall: {
    label: 'Reinstall',
    action: 'Reinstall',
    running: 'Reinstalling…',
    done: 'reinstalled',
    icon: RefreshCw,
    tone: 'text-muted-foreground',
  },
  install: {
    label: 'Install',
    action: 'Install',
    running: 'Installing…',
    done: 'installed',
    icon: Download,
    tone: 'text-muted-foreground',
  },
}

type Step = 'manifest' | 'install'

type Phase =
  | { kind: 'idle' }
  | { kind: 'running'; step: Step; to: Release; direction: Direction }
  | { kind: 'done'; to: Release; direction: Direction }
  | { kind: 'error'; title: string; description: string }

/** Go zero dates and unparsable strings both come out as non-positive or NaN. */
const validTime = (date: string): number | null => {
  const time = Date.parse(date)
  return time > 0 ? time : null
}

const plural = (count: number, unit: string) => `${count} ${unit}${count === 1 ? '' : 's'}`

const releaseAge = (publishedAt: string): string | null => {
  const time = validTime(publishedAt)
  if (time === null) return null
  const days = Math.floor((Date.now() - time) / DAY_MS)
  if (days < 1) return 'today'
  if (days < 14) return `${plural(days, 'day')} ago`
  if (days < 60) return `${plural(Math.floor(days / 7), 'week')} ago`
  if (days < 730) return `${plural(Math.floor(days / 30), 'month')} ago`
  return `${plural(Math.floor(days / 365), 'year')} ago`
}

/** Newest first. Releases without a usable date keep the order the API gave them, at the end. */
const newestFirst = (releases: Release[]): Release[] =>
  [...releases].sort((a, b) => (validTime(b.published_at) ?? 0) - (validTime(a.published_at) ?? 0))

const fetchReleases = async (addon: Addon): Promise<Release[]> => {
  try {
    const response = await apiClient.get(`/addon/${addon.name}/releases`)
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
    toast({
      title: 'Error',
      description: `Failed to load versions: ${errorMessage}`,
    })
    throw e
  }
}

export const LocalAddonVersionSelectDialog = ({ addon }: Props) => {
  const setAddon = useSetAtom(versionSelectAtom)
  const [selectedTag, setSelectedTag] = useState<string | null>(null)
  const [filter, setFilter] = useState('')
  const [phase, setPhase] = useState<Phase>({ kind: 'idle' })
  const iconUrl = useAtomValue(catalogIconMapAtom).get(addon.name) ?? null
  const listRef = useRef<HTMLDivElement>(null)
  const releasesQuery = useQuery({
    queryKey: ['addon-releases', addon.name],
    retry: false,
    queryFn: () => fetchReleases(addon),
  })
  const { update } = useAddonStore()

  const releases = newestFirst(releasesQuery.data ?? [])
  const installedIndex = releases.findIndex(release => release.tag_name === addon.version)
  const found = releases.findIndex(release => release.tag_name === selectedTag)
  const selectedIndex = found === -1 ? 0 : found
  const selected: Release | undefined = releases[selectedIndex]

  const query = filter.trim().toLowerCase()
  const visible = releases
    .map((release, index) => ({ release, index }))
    .filter(({ release }) => !query || release.tag_name.toLowerCase().includes(query))

  const direction: Direction =
    installedIndex === -1
      ? 'install'
      : selectedIndex < installedIndex
        ? 'upgrade'
        : selectedIndex > installedIndex
          ? 'rollback'
          : 'reinstall'
  const { action, icon: ActionIcon } = directions[direction]

  useEffect(() => {
    listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' })
  }, [selected?.tag_name])

  useEffect(() => {
    if (phase.kind !== 'done') return
    const timer = setTimeout(() => setAddon(null), AUTO_CLOSE_MS)
    return () => clearTimeout(timer)
  }, [phase.kind, setAddon])

  const isRunning = phase.kind === 'running'
  const isLocked = isRunning || phase.kind === 'done'

  const select = (tag: string) => {
    if (isLocked) return
    setSelectedTag(tag)
    setPhase({ kind: 'idle' })
  }

  const setOpen = (open: boolean) => {
    if (!open && !isRunning) {
      setAddon(null)
    }
  }

  const handleInstall = async () => {
    if (!selected || isLocked) return
    // A refetch can reorder the list under us, so pin what we started with.
    const snapshot = { to: selected, direction }

    setPhase({ kind: 'running', step: 'manifest', ...snapshot })
    const [result, err] = await safeCall(
      (async () => {
        const manifest = await repoGetManifest(addon.name)
        setPhase({ kind: 'running', step: 'install', ...snapshot })
        return update(manifest, selected.tag_name)
      })()
    )

    if (err || !result) {
      const message = err?.message ?? 'Unknown error'
      setPhase({
        kind: 'error',
        title: 'Install failed',
        description: message.includes('not found')
          ? 'No release found for this addon'
          : `Failed to install addon: ${message.substring(0, 100)}`,
      })
      return
    }

    notifyDependencyProgress(result)
    const failure = dependencyFailure(result, `Failed to install ${addon.alias}`)
    if (failure) {
      setPhase({
        kind: 'error',
        title:
          failure.kind === 'dependencies' ? 'Dependency installation failed' : 'Install failed',
        description: failure.description,
      })
      return
    }

    setPhase({ kind: 'done', ...snapshot })
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // Portaled children (the release notes image viewer) bubble here through React, not the DOM.
    if (!(event.target instanceof Element) || !event.currentTarget.contains(event.target)) return
    if (isLocked) return

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      if (visible.length === 0) return
      event.preventDefault()
      const at = visible.findIndex(({ index }) => index === selectedIndex)
      const next = at === -1 ? 0 : at + (event.key === 'ArrowDown' ? 1 : -1)
      select(visible[Math.max(0, Math.min(visible.length - 1, next))].release.tag_name)
    } else if (event.key === 'Enter' && !event.target.closest('button, a')) {
      event.preventDefault()
      handleInstall()
    }
  }

  return (
    <Dialog onOpenChange={setOpen} open={addon !== null}>
      <DialogContent
        showCloseButton={!isRunning}
        onKeyDown={handleKeyDown}
        className="flex max-h-[90vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-180"
      >
        <AnimatePresence initial={false} mode="wait">
          {phase.kind === 'done' ? (
            <motion.div
              key="done"
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              className="relative flex flex-col items-center px-6 pt-12 pb-10 text-center"
            >
              <div className="relative">
                <AddonIcon name={addon.alias} src={iconUrl} size="lg" className="opacity-90" />
                <motion.span
                  initial={{ scale: 0.4, opacity: 0 }}
                  animate={{ scale: 1, opacity: 1 }}
                  transition={{ type: 'spring', stiffness: 420, damping: 16, delay: 0.1 }}
                  className="absolute -right-2 -bottom-2 flex size-7 items-center justify-center rounded-full bg-green-500 text-background ring-4 ring-background"
                >
                  <Check className="size-4" strokeWidth={3} />
                </motion.span>
              </div>
              <DialogTitle className="mt-6 text-lg">
                {addon.alias} {directions[phase.direction].done}
              </DialogTitle>
              <DialogDescription className="mt-2 font-mono">
                {phase.direction === 'reinstall' || !addon.version
                  ? phase.to.tag_name
                  : `${addon.version} → ${phase.to.tag_name}`}
              </DialogDescription>
              <div
                className="mt-7 h-1 w-40 overflow-hidden rounded-full bg-border"
                aria-hidden="true"
              >
                <motion.div
                  className="h-full bg-green-500"
                  initial={{ width: '0%' }}
                  animate={{ width: '100%' }}
                  transition={{ duration: AUTO_CLOSE_MS / 1000, ease: 'linear' }}
                />
              </div>
            </motion.div>
          ) : (
            <motion.div
              key="select"
              className="relative flex min-h-0 flex-1 flex-col"
              exit={{ opacity: 0, scale: 0.98 }}
              transition={{ duration: 0.15 }}
            >
              <div className="flex shrink-0 items-end justify-between gap-4 px-6 pt-5 pr-14 pb-4">
                <div className="min-w-0">
                  <DialogDescription className={cn(eyebrowClass, 'mb-1 text-primary')}>
                    Install version
                  </DialogDescription>
                  <DialogTitle className="truncate text-xl leading-tight tracking-tight">
                    {addon.alias}
                  </DialogTitle>
                </div>
                <div className="flex shrink-0 items-end gap-2.5 font-mono text-xs">
                  <div className="text-muted-foreground">
                    <p className={cn(eyebrowClass, 'font-sans text-muted-foreground/70')}>
                      Installed
                    </p>
                    <p title={addon.version} className="max-w-28 truncate">
                      {addon.version || '—'}
                    </p>
                  </div>
                  {selected && (
                    <>
                      <ArrowRight
                        className="mb-0.5 size-4 text-muted-foreground/60"
                        aria-hidden="true"
                      />
                      <div className="text-right font-semibold">
                        <p className={cn(eyebrowClass, 'font-sans', directions[direction].tone)}>
                          {directions[direction].label}
                        </p>
                        <p title={selected.tag_name} className="max-w-28 truncate">
                          {selected.tag_name}
                        </p>
                      </div>
                    </>
                  )}
                </div>
              </div>

              <div
                inert={isRunning}
                className={cn(
                  'flex h-93 min-h-0 shrink border-y border-border/60 transition-opacity',
                  isRunning && 'opacity-50'
                )}
              >
                {releasesQuery.isPending && <LoadingState />}
                {releasesQuery.isError && (
                  <CenteredState
                    icon={AlertCircle}
                    destructive
                    title="Couldn't load versions"
                    description={releasesQuery.error.message}
                  >
                    <Button
                      variant="outline"
                      size="sm"
                      className="mt-2"
                      onClick={() => releasesQuery.refetch()}
                    >
                      <RefreshCw />
                      Retry
                    </Button>
                  </CenteredState>
                )}
                {releasesQuery.isSuccess && !selected && (
                  <CenteredState
                    icon={TagIcon}
                    title="Nothing else to install"
                    description="This addon has no other published versions."
                  />
                )}
                {releasesQuery.isSuccess && selected && (
                  <>
                    <div className="flex w-64 shrink-0 flex-col border-r border-border/60 bg-muted/20">
                      <div className="relative shrink-0 p-2.5 pb-1.5">
                        <SearchIcon className="pointer-events-none absolute top-1/2 left-5 size-3.5 -translate-y-[calc(50%-2px)] text-muted-foreground" />
                        <Input
                          value={filter}
                          onChange={event => setFilter(event.target.value)}
                          placeholder="Filter versions"
                          aria-label="Filter versions"
                          spellCheck={false}
                          className="h-8 pl-8 text-xs md:text-xs"
                        />
                      </div>
                      <ScrollArea className="min-h-0 flex-1">
                        <div
                          ref={listRef}
                          role="listbox"
                          aria-label="Available versions"
                          className="px-2 pt-0.5 pb-2.5"
                        >
                          {visible.length === 0 && (
                            <p className="px-3 py-7 text-center text-xs text-muted-foreground">
                              No version matches that filter.
                            </p>
                          )}
                          {visible.map(({ release, index }, position) => (
                            <VersionRow
                              key={release.tag_name}
                              release={release}
                              selected={index === selectedIndex}
                              installed={index === installedIndex}
                              newer={index < installedIndex}
                              latest={index === 0}
                              first={position === 0}
                              last={position === visible.length - 1}
                              onSelect={() => select(release.tag_name)}
                            />
                          ))}
                        </div>
                      </ScrollArea>
                    </div>
                    <ReleaseDetail
                      release={selected}
                      direction={direction}
                      distance={Math.abs(selectedIndex - installedIndex)}
                      latest={selectedIndex === 0}
                    />
                  </>
                )}
              </div>

              {phase.kind === 'error' && (
                <motion.div
                  role="alert"
                  initial={{ opacity: 0, y: -4 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="mx-6 mt-4 flex shrink-0 gap-3 rounded-xl border border-destructive/40 bg-destructive/10 p-3"
                >
                  <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
                  <div className="min-w-0">
                    <p className="text-sm font-medium">{phase.title}</p>
                    <p className="text-sm wrap-break-word text-muted-foreground">
                      {phase.description}
                    </p>
                  </div>
                </motion.div>
              )}

              {phase.kind === 'running' ? (
                <div className="shrink-0 space-y-3 px-6 py-4" aria-live="polite">
                  {[
                    {
                      label: 'Resolving manifest',
                      state: phase.step === 'manifest' ? 'active' : 'done',
                    },
                    {
                      label: `Installing ${phase.to.tag_name}`,
                      state: phase.step === 'install' ? 'active' : 'pending',
                    },
                  ].map(({ label, state }) => (
                    <div key={label} className="flex items-center gap-3 text-sm">
                      {state === 'done' ? (
                        <span className="flex size-5 items-center justify-center rounded-full bg-green-500/15">
                          <Check className="size-3 text-green-500" strokeWidth={3} />
                        </span>
                      ) : state === 'active' ? (
                        <LoaderCircle className="size-5 animate-spin text-primary" />
                      ) : (
                        <span className="size-5 rounded-full border-2 border-border" />
                      )}
                      <span
                        className={cn(
                          state === 'pending' ? 'text-muted-foreground' : 'text-foreground',
                          state === 'active' && 'font-medium'
                        )}
                      >
                        {label}
                      </span>
                      <span className="ml-auto text-xs text-muted-foreground">
                        {state === 'done' ? 'done' : state === 'active' ? '…' : ''}
                      </span>
                    </div>
                  ))}
                  <div className="relative h-10 overflow-hidden rounded-xl bg-primary/10 ring-1 ring-primary/20 ring-inset">
                    <motion.div
                      className="absolute inset-y-0 w-1/2 bg-linear-to-r from-transparent via-primary/35 to-transparent"
                      initial={{ x: '-100%' }}
                      animate={{ x: '200%' }}
                      transition={{ repeat: Infinity, duration: 1.4, ease: 'easeInOut' }}
                    />
                    <span className="relative flex h-full items-center justify-center text-sm font-medium text-primary">
                      {directions[phase.direction].running}
                    </span>
                  </div>
                </div>
              ) : (
                <div className="flex shrink-0 items-center justify-between gap-3 px-6 py-3.5 pr-4">
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground/80">
                    <kbd className={kbdClass}>↑</kbd>
                    <kbd className={kbdClass}>↓</kbd>
                    <span className="mr-1.5">browse</span>
                    <kbd className={kbdClass}>↵</kbd>
                    <span>install</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button variant="ghost" onClick={() => setOpen(false)}>
                      Cancel
                    </Button>
                    <Button
                      onClick={handleInstall}
                      disabled={!selected}
                      className="rounded-lg font-semibold shadow-lg shadow-primary/20"
                    >
                      {selected ? (
                        <>
                          <ActionIcon />
                          <span>
                            {action} <span className="font-mono">{selected.tag_name}</span>
                          </span>
                        </>
                      ) : (
                        'Install'
                      )}
                    </Button>
                  </div>
                </div>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </DialogContent>
    </Dialog>
  )
}

interface VersionRowProps {
  release: Release
  selected: boolean
  installed: boolean
  /** Sits above the installed version on the rail. */
  newer: boolean
  latest: boolean
  first: boolean
  last: boolean
  onSelect: () => void
}

const VersionRow = ({
  release,
  selected,
  installed,
  newer,
  latest,
  first,
  last,
  onSelect,
}: VersionRowProps) => (
  <div
    role="option"
    aria-selected={selected}
    onClick={onSelect}
    className="group relative grid cursor-pointer grid-cols-[22px_1fr] items-center py-2.5 pr-2 pl-1"
  >
    {/* The highlight is inset so neighbours never touch, while the rail still runs row to row. */}
    <span
      aria-hidden="true"
      className={cn(
        'absolute inset-x-0 inset-y-0.5 rounded-lg transition-colors',
        selected ? 'bg-card ring-1 ring-border ring-inset' : 'group-hover:bg-foreground/4'
      )}
    />
    <span
      aria-hidden="true"
      className={cn(
        'absolute left-3.75 w-px -translate-x-1/2',
        first ? 'top-1/2' : 'top-0',
        last ? 'bottom-1/2' : 'bottom-0',
        newer ? 'bg-primary/40' : 'bg-border',
        installed && 'bg-linear-to-b from-primary/40 from-50% to-border to-50%'
      )}
    />
    <span
      aria-hidden="true"
      className={cn(
        'relative justify-self-center rounded-full transition-all',
        installed
          ? 'size-2.25 bg-primary ring-4 ring-primary/15'
          : 'size-1.75 border-[1.5px] bg-background',
        !installed && (selected ? 'scale-125 border-foreground' : 'border-muted-foreground/60')
      )}
    />
    <div className="relative min-w-0 pl-1.5">
      <div className="flex items-center gap-1.5">
        <span
          title={release.tag_name}
          className={cn(
            'truncate font-mono text-[13px] font-medium',
            selected || installed ? 'text-foreground' : 'text-muted-foreground'
          )}
        >
          {release.tag_name}
        </span>
        {installed && <RowBadge className="bg-primary/15 text-primary">Installed</RowBadge>}
        {latest && <RowBadge className="bg-emerald-500/15 text-emerald-500">Latest</RowBadge>}
      </div>
      <p className="mt-px text-[11px] text-muted-foreground/70">
        {releaseAge(release.published_at) ?? ' '}
      </p>
    </div>
  </div>
)

const RowBadge = ({ className, children }: { className: string; children: string }) => (
  <span
    className={cn(
      'shrink-0 rounded-full px-1.5 py-px text-[9px] leading-normal font-semibold tracking-wider uppercase',
      className
    )}
  >
    {children}
  </span>
)

interface ReleaseDetailProps {
  release: Release
  direction: Direction
  /** How many releases away from the installed one. */
  distance: number
  latest: boolean
}

const ReleaseDetail = ({ release, direction, distance, latest }: ReleaseDetailProps) => {
  const changelog = release.body?.trim() ?? ''
  const summary =
    direction === 'reinstall'
      ? 'This is the version you have installed'
      : direction === 'install'
        ? 'Your installed version is not in this list'
        : `${plural(distance, 'release')} ${direction === 'upgrade' ? 'newer' : 'older'} than installed`

  return (
    <div className="flex min-w-0 flex-1 flex-col px-6 pt-5">
      <div className="flex shrink-0 items-baseline justify-between gap-3">
        <p
          title={release.tag_name}
          className="truncate font-mono text-2xl leading-none font-semibold tracking-tight"
        >
          {release.tag_name}
        </p>
        {validTime(release.published_at) !== null && (
          <p className="shrink-0 text-xs text-muted-foreground">
            Released {formatToLocalDate(release.published_at, 'short')}
          </p>
        )}
      </div>
      <p className="mt-2.5 flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
        <span
          aria-hidden="true"
          className={cn(
            'size-1.5 rounded-full',
            direction === 'upgrade'
              ? 'bg-emerald-500'
              : direction === 'install'
                ? 'bg-muted-foreground/60'
                : 'bg-primary'
          )}
        />
        {summary}
        {latest && direction !== 'reinstall' && ' · latest'}
      </p>

      <ScrollArea className="mt-3.5 min-h-0 flex-1 border-t border-dashed border-border">
        <div
          key={release.tag_name}
          className="animate-in pt-3.5 pr-3 pb-5 duration-200 fade-in-0 slide-in-from-bottom-1"
        >
          {changelog ? (
            <div className="release-notes wrap-break-word">
              <Readme readme={changelog} />
            </div>
          ) : (
            <p className="text-sm text-muted-foreground/70 italic">
              No release notes were published for this version.
            </p>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}

const LoadingState = () => (
  <>
    <div className="w-64 shrink-0 space-y-4 border-r border-border/60 bg-muted/20 p-4">
      <Skeleton className="h-8 w-full" />
      {['w-24', 'w-20', 'w-28', 'w-20', 'w-24'].map((width, index) => (
        <div key={index} className="flex items-center gap-3 pl-1">
          <Skeleton className="size-2 rounded-full" />
          <div className="space-y-1.5">
            <Skeleton className={cn('h-3', width)} />
            <Skeleton className="h-2 w-14" />
          </div>
        </div>
      ))}
    </div>
    <div className="flex-1 space-y-3 px-6 pt-5">
      <Skeleton className="h-6 w-28" />
      <Skeleton className="h-3 w-48" />
      <div className="space-y-2.5 pt-5">
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-5/6" />
        <Skeleton className="h-3 w-2/3" />
      </div>
      <span className="sr-only">Loading versions...</span>
    </div>
  </>
)

const CenteredState = ({
  icon: Icon,
  destructive,
  title,
  description,
  children,
}: {
  icon: ComponentType<{ className?: string }>
  destructive?: boolean
  title: string
  description: string
  children?: React.ReactNode
}) => (
  <div className="flex flex-1 flex-col items-center justify-center gap-1 px-6 text-center">
    <span
      className={cn(
        'mb-2 flex size-10 items-center justify-center rounded-full border',
        destructive
          ? 'border-destructive/30 bg-destructive/10 text-destructive'
          : 'border-border bg-card text-muted-foreground'
      )}
    >
      <Icon className="size-4.5" />
    </span>
    <p className="text-sm font-medium">{title}</p>
    <p className="max-w-xs text-sm wrap-break-word text-muted-foreground">{description}</p>
    {children}
  </div>
)
