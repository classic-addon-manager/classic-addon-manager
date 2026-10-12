import { Browser } from '@wailsio/runtime'
import { AnimatePresence, motion } from 'framer-motion'
import { useAtom, useAtomValue } from 'jotai'
import {
  AlertTriangle,
  ArrowUpRight,
  Boxes,
  Check,
  ChevronRight,
  Download,
  GitBranch,
  GitCommitHorizontal,
  LoaderCircle,
  RotateCcw,
} from 'lucide-react'
import { type ComponentType, type ReactNode, useEffect, useState } from 'react'
import semver from 'semver'

import { localUpdateDialogOpenAtom } from '@/components/dashboard/atoms'
import { catalogIconMapAtom } from '@/components/dashboard/iconMap'
import { AddonIcon } from '@/components/shared/AddonIcon'
import { Readme } from '@/components/shared/Readme'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { dependencyFailure, notifyDependencyProgress } from '@/lib/notifyDependencyResult'
import { repoGetManifest } from '@/lib/repo.ts'
import { cn, formatToLocalDate, safeCall, timeAgo } from '@/lib/utils'
import type { Addon, Release } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

interface Props {
  addon: Addon
  release: Release
}

/** How long the success state stays up before the dialog closes itself. */
const AUTO_CLOSE_MS = 1800

const DAY_MS = 86_400_000

const eyebrowClass =
  'mb-1 h-4 text-[10px] leading-4 font-semibold tracking-[0.14em] text-muted-foreground uppercase'

const kbdClass =
  'rounded-md border border-border px-1.5 py-0.5 font-sans text-[10px] leading-none text-muted-foreground'

type Step = 'manifest' | 'install'

type Phase =
  | { kind: 'idle' }
  | { kind: 'running'; step: Step; from: Addon; to: Release }
  | { kind: 'done'; from: Addon; to: Release }
  | { kind: 'error'; title: string; description: string }

type Bump = 'major' | 'minor' | 'patch' | 'pre-release' | 'republished'

const bumpLabels: Record<Bump, string> = {
  major: 'Major update',
  minor: 'Minor update',
  patch: 'Patch',
  'pre-release': 'Pre-release',
  republished: 'Re-published',
}

/** Classifies the jump between two tags, or null when they are not comparable versions. */
const releaseBump = (from: string, to: string): Bump | null => {
  if (from === to) return 'republished'
  const installed = semver.clean(from, { loose: true })
  const available = semver.clean(to, { loose: true })
  if (!installed || !available || !semver.gt(available, installed)) return null
  const diff = semver.diff(installed, available)
  return diff === 'major' || diff === 'minor' || diff === 'patch' ? diff : 'pre-release'
}

/** Go zero dates and unparsable strings both come out as non-positive or NaN. */
const validTime = (date: string): number | null => {
  const time = Date.parse(date)
  return time > 0 ? time : null
}

const releaseGapLabel = (installedAt: string, publishedAt: string): string | null => {
  const from = validTime(installedAt)
  const to = validTime(publishedAt)
  if (from === null || to === null || to < from) return null
  const days = Math.floor((to - from) / DAY_MS)
  if (days === 0) return 'same day'
  return days === 1 ? '+1 day' : `+${days} days`
}

export const LocalAddonUpdateDialog = ({ addon, release }: Props) => {
  const [open, setOpen] = useAtom(localUpdateDialogOpenAtom)
  const { update } = useAddonStore()
  const iconUrl = useAtomValue(catalogIconMapAtom).get(addon.name) ?? null
  const [phase, setPhase] = useState<Phase>({ kind: 'idle' })

  // Reopening starts fresh, adjusting during render avoids an effect and a stale frame.
  const [wasOpen, setWasOpen] = useState(open)
  if (open !== wasOpen) {
    setWasOpen(open)
    if (open) setPhase({ kind: 'idle' })
  }

  useEffect(() => {
    if (phase.kind !== 'done') return
    const timer = setTimeout(() => setOpen(false), AUTO_CLOSE_MS)
    return () => clearTimeout(timer)
  }, [phase.kind, setOpen])

  // Once the update starts the store refreshes the addon under us, so pin what we started from.
  const pinned = phase.kind === 'running' || phase.kind === 'done'
  const from = pinned ? phase.from : addon
  const to = pinned ? phase.to : release

  const handleOpenChange = (next: boolean) => {
    if (!next && phase.kind === 'running') return
    setOpen(next)
  }

  const handleUpdate = async () => {
    if (phase.kind === 'running') return
    const snapshot = { from: addon, to: release }

    setPhase({ kind: 'running', step: 'manifest', ...snapshot })
    const [result, err] = await safeCall(
      (async () => {
        const manifest = await repoGetManifest(addon.name)
        setPhase({ kind: 'running', step: 'install', ...snapshot })
        return update(manifest, release.tag_name)
      })()
    )

    if (err || !result) {
      const message = err?.message ?? 'Unknown error'
      setPhase({
        kind: 'error',
        title: 'Update failed',
        description: message.includes('not found')
          ? 'No release found for this addon'
          : `Failed to update addon: ${message.substring(0, 100)}`,
      })
      return
    }

    notifyDependencyProgress(result)
    const failure = dependencyFailure(result, `Failed to update ${addon.alias}`)
    if (failure) {
      setPhase({
        kind: 'error',
        title: failure.kind === 'dependencies' ? 'Dependency installation failed' : 'Update failed',
        description: failure.description,
      })
      return
    }

    setPhase({ kind: 'done', ...snapshot })
  }

  return (
    <Dialog onOpenChange={handleOpenChange} open={open}>
      <DialogContent
        showCloseButton={phase.kind !== 'running'}
        className="flex max-h-[85vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-lg"
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
                <AddonIcon name={from.alias} src={iconUrl} size="lg" className="opacity-90" />
                <motion.span
                  initial={{ scale: 0.4, opacity: 0 }}
                  animate={{ scale: 1, opacity: 1 }}
                  transition={{ type: 'spring', stiffness: 420, damping: 16, delay: 0.1 }}
                  className="absolute -right-2 -bottom-2 flex size-7 items-center justify-center rounded-full bg-green-500 text-background ring-4 ring-background"
                >
                  <Check className="size-4" strokeWidth={3} />
                </motion.span>
              </div>
              <DialogTitle className="mt-6 text-lg">{from.alias} is up to date</DialogTitle>
              <DialogDescription className="mt-2 font-mono">
                {from.version} → {to.tag_name}
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
              key="update"
              className="relative flex min-h-0 flex-1 flex-col"
              exit={{ opacity: 0, scale: 0.98 }}
              transition={{ duration: 0.15 }}
            >
              <UpdateHero addon={from} release={to} iconUrl={iconUrl} />
              <ReleaseNotes repo={from.repo} release={to} />

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

              <div className="mt-5 shrink-0 border-t border-border/60 bg-muted/20 px-6 py-4">
                {phase.kind === 'running' ? (
                  <div className="space-y-3" aria-live="polite">
                    {[
                      {
                        label: 'Resolving manifest',
                        state: phase.step === 'manifest' ? 'active' : 'done',
                      },
                      {
                        label: `Installing ${to.tag_name}`,
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
                        Updating…
                      </span>
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <Button variant="ghost" onClick={() => setOpen(false)}>
                      Later
                      <kbd className={kbdClass}>Esc</kbd>
                    </Button>
                    <Button
                      autoFocus
                      onClick={handleUpdate}
                      className="ml-auto h-10 min-w-52 rounded-xl px-4 shadow-lg shadow-primary/20"
                    >
                      {phase.kind === 'error' ? <RotateCcw /> : <Download />}
                      <span className="truncate">
                        {phase.kind === 'error' ? 'Retry update' : `Update to ${to.tag_name}`}
                      </span>
                      <kbd
                        className={cn(
                          kbdClass,
                          'ml-auto border-primary-foreground/20 bg-primary-foreground/10 text-primary-foreground/80'
                        )}
                      >
                        ↵
                      </kbd>
                    </Button>
                  </div>
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </DialogContent>
    </Dialog>
  )
}

interface UpdateHeroProps {
  addon: Addon
  release: Release
  iconUrl: string | null
}

const UpdateHero = ({ addon, release, iconUrl }: UpdateHeroProps) => {
  const bump = releaseBump(addon.version, release.tag_name)
  const gap = releaseGapLabel(addon.updatedAt, release.published_at)
  const installedAt = validTime(addon.updatedAt)
    ? formatToLocalDate(addon.updatedAt, 'short')
    : null
  const publishedAt = validTime(release.published_at)
    ? `${formatToLocalDate(release.published_at, 'short')} · ${timeAgo(release.published_at)}`
    : null
  const byline = [addon.author && `by ${addon.author}`, addon.repo].filter(Boolean).join(' · ')
  const dependencyCount = addon.dependencies?.length ?? 0

  return (
    <div className="shrink-0 px-6 pt-6">
      <div className="flex items-start gap-4 pr-8">
        <AddonIcon
          name={addon.alias}
          src={iconUrl}
          size="lg"
          className="shadow-lg shadow-primary/10 ring-1 ring-primary/30"
        />
        <div className="min-w-0 flex-1 pt-1">
          <DialogTitle className="truncate text-xl leading-tight">{addon.alias}</DialogTitle>
          <DialogDescription className="mt-1 truncate">
            {byline || 'Update available'}
          </DialogDescription>
        </div>
        {bump && (
          <span className="mt-1.5 shrink-0 rounded-full bg-primary/15 px-2.5 py-1 text-[10px] font-bold tracking-widest text-primary uppercase">
            {bumpLabels[bump]}
          </span>
        )}
      </div>

      <div className="mt-6 grid grid-cols-[auto_minmax(3rem,1fr)_auto] items-start gap-4">
        <div className="min-w-0">
          <p className={eyebrowClass}>Installed</p>
          <p
            title={addon.version}
            className="flex h-9 max-w-36 items-center truncate font-mono text-2xl text-muted-foreground"
          >
            {addon.version || '—'}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">{installedAt ?? '\u00a0'}</p>
        </div>

        <div className="relative mt-7.5 flex items-center" aria-hidden="true">
          <div className="h-px flex-1 border-t border-dashed border-primary/50" />
          <ChevronRight className="-ml-1.5 size-4 shrink-0 text-primary" />
          {gap && (
            <span className="absolute left-1/2 -translate-x-1/2 rounded-full border border-border bg-background px-2 py-0.5 text-[11px] whitespace-nowrap text-muted-foreground">
              {gap}
            </span>
          )}
        </div>

        <div className="min-w-0">
          <p className={cn(eyebrowClass, 'text-primary')}>Available</p>
          <p
            title={release.tag_name}
            className="flex h-9 max-w-48 items-center truncate font-mono text-3xl font-bold tracking-tight"
          >
            {release.tag_name}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">{publishedAt ?? '\u00a0'}</p>
        </div>
      </div>

      {(dependencyCount > 0 || addon.commit || addon.branch) && (
        <div className="mt-5 flex flex-wrap gap-2">
          {dependencyCount > 0 && (
            <Chip icon={Boxes}>
              {dependencyCount} {dependencyCount === 1 ? 'dependency' : 'dependencies'}
            </Chip>
          )}
          {addon.commit && (
            <Chip icon={GitCommitHorizontal} title="Installed commit" mono>
              {addon.commit.slice(0, 7)}
            </Chip>
          )}
          {addon.branch && <Chip icon={GitBranch}>{addon.branch}</Chip>}
        </div>
      )}
    </div>
  )
}

const ReleaseNotes = ({ repo, release }: { repo: string; release: Release }) => {
  const changelog = release.body?.trim() ?? ''
  const hasSections = /^#{1,6}\s/m.test(changelog)

  return (
    <div className="mt-5 flex min-h-0 flex-1 flex-col border-t border-border/60 px-6 pt-4">
      <div className="mb-3 flex shrink-0 items-center justify-between">
        <h3 className="text-sm font-semibold">What's new</h3>
        {repo && (
          <button
            type="button"
            onClick={() =>
              Browser.OpenURL(
                `https://github.com/${repo}/releases/tag/${encodeURIComponent(release.tag_name)}`
              )
            }
            className="inline-flex items-center gap-1 rounded-sm text-xs text-primary hover:underline focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none"
          >
            Full changelog
            <ArrowUpRight className="size-3.5" />
          </button>
        )}
      </div>

      {changelog ? (
        <div className="flex min-h-0 flex-col rounded-xl border border-border/60 bg-card/40">
          {/* Sized by content up to max-h-64, and shrinks further when the dialog hits 85vh. */}
          <ScrollArea className="flex max-h-64 min-h-0 flex-col mask-[linear-gradient(to_bottom,black_calc(100%-2rem),transparent)] [&>[data-slot=scroll-area-viewport]]:h-auto [&>[data-slot=scroll-area-viewport]]:min-h-0">
            <div
              className={cn(
                'release-notes p-4 pb-8 wrap-break-word',
                hasSections && 'release-notes-timeline'
              )}
            >
              <Readme readme={changelog} />
            </div>
          </ScrollArea>
        </div>
      ) : (
        <p className="rounded-xl border border-dashed border-border/60 px-4 py-6 text-center text-sm text-muted-foreground">
          No release notes provided.
        </p>
      )}
    </div>
  )
}

const Chip = ({
  icon: Icon,
  title,
  mono,
  children,
}: {
  icon: ComponentType<{ className?: string }>
  title?: string
  mono?: boolean
  children: ReactNode
}) => (
  <span
    title={title}
    className={cn(
      'inline-flex items-center gap-1.5 rounded-lg border border-border/70 bg-background/40 px-2.5 py-1 text-xs text-muted-foreground',
      mono && 'font-mono'
    )}
  >
    <Icon className="size-3.5" />
    {children}
  </span>
)
