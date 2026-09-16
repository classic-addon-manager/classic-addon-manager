import { LoaderCircle, Plus, RefreshCw } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

import { Button } from '@/components/ui/button'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { useTitleBarSlot } from '@/hooks/useTitleBarSlot'
import { cn } from '@/lib/utils'

import type { OwnedAddonsData } from './useOwnedAddons'

export type DeveloperScope = 'all' | 'addons' | 'submissions'

const HIGHLIGHT_TIMEOUT_MS = 60_000

export function DeveloperToolbar({
  data,
  lastAttemptFailed,
  onRefresh,
  onNewAddon,
  scope,
  onScopeChange,
}: {
  data: OwnedAddonsData
  lastAttemptFailed: boolean
  onRefresh: () => Promise<void>
  onNewAddon: () => void
  scope: DeveloperScope
  onScopeChange: (scope: DeveloperScope) => void
}) {
  const [showSpinner, setShowSpinner] = useState(false)
  const spinnerTimerRef = useRef<number | null>(null)

  useEffect(
    () => () => {
      if (spinnerTimerRef.current !== null) window.clearTimeout(spinnerTimerRef.current)
    },
    []
  )

  // Same min-duration hold as AddonsToolbar: a click must read as feedback even
  // when the fetch resolves in milliseconds. Poll-triggered fetches stay silent.
  const handleRefresh = async () => {
    if (showSpinner) return
    setShowSpinner(true)
    const startTime = Date.now()
    try {
      await onRefresh()
    } finally {
      const remaining = Math.max(0, 500 - (Date.now() - startTime))
      spinnerTimerRef.current = window.setTimeout(() => {
        spinnerTimerRef.current = null
        setShowSpinner(false)
      }, remaining)
    }
  }

  const [highlightSubmissions, setHighlightSubmissions] = useState(false)
  const previousSubmissionsRef = useRef<Set<string> | null>(null)
  const highlightTimerRef = useRef<number | null>(null)

  // Highlight the Submissions segment when a poll adds a submission or changes its status.
  useEffect(() => {
    const signatures = new Set(
      data.submissions.map(submission => `${submission.id}:${submission.status}`)
    )
    const previous = previousSubmissionsRef.current
    previousSubmissionsRef.current = signatures
    if (previous === null) return
    const hasNewOrChanged = data.submissions.some(
      submission => !previous.has(`${submission.id}:${submission.status}`)
    )
    if (!hasNewOrChanged) return
    setHighlightSubmissions(true)
    if (highlightTimerRef.current !== null) window.clearTimeout(highlightTimerRef.current)
    highlightTimerRef.current = window.setTimeout(() => {
      setHighlightSubmissions(false)
      highlightTimerRef.current = null
    }, HIGHLIGHT_TIMEOUT_MS)
  }, [data])

  useEffect(
    () => () => {
      if (highlightTimerRef.current !== null) window.clearTimeout(highlightTimerRef.current)
    },
    []
  )

  const clearHighlight = () => {
    if (highlightTimerRef.current !== null) {
      window.clearTimeout(highlightTimerRef.current)
      highlightTimerRef.current = null
    }
    setHighlightSubmissions(false)
  }

  const handleScopeChange = (value: string) => {
    if (!value) return
    clearHighlight()
    onScopeChange(value as DeveloperScope)
  }

  // Same visibility rule as the workspace nav: 'update' submissions whose addon
  // is already published are folded into that addon and hidden from the count.
  const publishedNames = new Set(data.addons.map(addon => addon.name))
  const visibleSubmissionCount = data.submissions.filter(
    submission => !(submission.kind === 'update' && publishedNames.has(submission.payload.name))
  ).length

  const toolbarContent = (
    <div className="flex w-full min-w-0 items-center justify-between gap-3">
      <div className="no-drag flex shrink-0 items-center gap-2">
        <ToggleGroup
          type="single"
          variant="outline"
          size="sm"
          value={scope}
          onValueChange={handleScopeChange}
        >
          <ToggleGroupItem value="all">All</ToggleGroupItem>
          <ToggleGroupItem value="addons">Addons ({data.addons.length})</ToggleGroupItem>
          <ToggleGroupItem
            value="submissions"
            className={cn(highlightSubmissions && 'bg-primary/20 text-primary')}
          >
            Submissions ({visibleSubmissionCount})
          </ToggleGroupItem>
        </ToggleGroup>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <div className="no-drag flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={showSpinner}
            className={cn(
              'h-8 transition-all duration-200 hover:shadow-md',
              lastAttemptFailed &&
                'text-destructive border-destructive/40 bg-destructive/10 hover:bg-destructive/20'
            )}
            onClick={handleRefresh}
          >
            {showSpinner ? (
              <LoaderCircle className="mr-1.5 size-3.5 animate-spin" />
            ) : (
              <RefreshCw className="mr-1.5 size-3.5" />
            )}
            Refresh
          </Button>

          <Button type="button" size="sm" className="h-8" onClick={onNewAddon}>
            <Plus className="size-3.5" />
            <span className="hidden sm:inline">New addon</span>
          </Button>
        </div>
      </div>
    </div>
  )

  useTitleBarSlot(toolbarContent)

  return null
}
