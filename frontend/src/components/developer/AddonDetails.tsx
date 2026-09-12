import { Browser } from '@wailsio/runtime'
import { GithubIcon, LoaderCircle } from 'lucide-react'
import { useState } from 'react'

import { AddonEditPanel } from '@/components/developer/AddonEditPanel'
import { AddonIcon } from '@/components/developer/AddonIcon'
import { AddonStatistics } from '@/components/developer/AddonStatistics'
import { backendUnavailable, latestReview } from '@/components/developer/catalogEditing'
import { DetailField } from '@/components/developer/DetailField'
import { valuesToForm } from '@/components/developer/formValues.ts'
import type { OwnedAddon } from '@/components/developer/ownedParse'
import { ReviewStatus } from '@/components/developer/ReviewStatus'
import { StatusChip } from '@/components/developer/StatusChip'
import { useDevAddonValues } from '@/components/developer/useDevAddonValues.ts'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { formatToLocalDate } from '@/lib/utils'

export function AddonDetails({
  addon,
  onSelect,
  onRefresh,
}: {
  addon: OwnedAddon
  onSelect: (key: string) => void
  onRefresh: () => Promise<void>
}) {
  const loaded = useDevAddonValues({
    type: 'addon',
    uuid: addon.uuid,
    name: addon.name,
    alias: addon.alias,
  })
  const form = loaded.values ? valuesToForm(loaded.values) : null
  const display: OwnedAddon = form
    ? {
        ...addon,
        alias: form.alias || addon.alias,
        repo: form.repo,
        branch: form.branch || null,
        author: form.author,
        description: form.description,
        tags: [...form.tags],
      }
    : addon
  const [tab, setTab] = useState('overview')
  const [mode, setMode] = useState<'view' | 'edit'>('view')
  // The addon's current status is its latest submission, never a previewed one.
  const review = latestReview(display.reviewHistory)
  const submitLabel =
    review?.status === 'in_review'
      ? 'Update submission'
      : review?.status === 'rejected'
        ? 'Resubmit for review'
        : 'Submit for review'
  const edit = () => {
    setTab('overview')
    setMode('edit')
  }

  return (
    <Tabs
      value={tab}
      onValueChange={value => {
        setMode('view')
        setTab(value)
        // TODO(backend): Load statistics on selection, including keyboard tab navigation.
        if (value === 'statistics') backendUnavailable('Addon statistics')
      }}
      className="h-full min-h-0 gap-0"
    >
      <div className="shrink-0 space-y-4 px-5 pt-5">
        <div className="flex items-start gap-3">
          <AddonIcon addon={display} />
          <div className="min-w-0">
            <h2 className="wrap-break-word text-xl font-semibold tracking-tight">
              {display.alias}
            </h2>
            <p className="mt-1 text-xs text-muted-foreground">
              Published{display.author && ` · ${display.author}`}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button onClick={edit} disabled={loaded.loading || !!loaded.error}>
            {review?.status === 'rejected'
              ? 'Revise changes'
              : review?.status === 'in_review'
                ? 'Edit pending changes'
                : 'Update details'}
          </Button>
          {display.repo && (
            <Button
              variant="outline"
              onClick={() => void Browser.OpenURL(`https://github.com/${display.repo}`)}
            >
              <GithubIcon />
              View code
            </Button>
          )}
        </div>
        <TabsList className="h-auto rounded-none border-b bg-transparent p-0">
          <TabsTrigger value="overview" className="rounded-none px-3 py-2">
            Overview
          </TabsTrigger>
          <TabsTrigger value="statistics" className="rounded-none px-3 py-2">
            Statistics
          </TabsTrigger>
          <TabsTrigger value="history" className="rounded-none px-3 py-2">
            Submission history
          </TabsTrigger>
        </TabsList>
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <TabsContent value="overview" className="space-y-5 p-5">
          {loaded.loading ? (
            <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
              <LoaderCircle className="size-8 animate-spin opacity-50" strokeWidth={1.5} />
              <p className="mt-3 text-sm">Loading declaration...</p>
            </div>
          ) : loaded.error ? (
            <p className="text-sm text-destructive">{loaded.error}</p>
          ) : mode === 'edit' ? (
            <AddonEditPanel
              addon={display}
              initialForm={form}
              submitLabel={submitLabel}
              onCancel={() => setMode('view')}
              onSubmitted={() => void onRefresh()}
            />
          ) : (
            <>
              {review && (
                <ReviewStatus
                  key={`${review.status}:${review.submissionId}`}
                  review={review}
                  onInspect={() => onSelect(`submission:${review.submissionId}`)}
                  onRefresh={onRefresh}
                />
              )}
              {display.warning && (
                <p className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm">
                  {display.warning}
                </p>
              )}
              <section className="space-y-3">
                <h3 className="text-sm font-medium">Listing details</h3>
                <p className="whitespace-pre-wrap wrap-break-word text-sm text-muted-foreground">
                  {display.description || 'No description provided.'}
                </p>
                <dl className="divide-y rounded-xl border bg-card/40 px-4 text-sm">
                  <DetailField
                    label="Repository"
                    value={display.repo}
                    href={display.repo !== '' ? `https://github.com/${display.repo}` : undefined}
                  />
                  <DetailField label="Branch" value={display.branch ?? 'Not specified'} />
                  <DetailField label="Tags" value={display.tags.join(', ')} />
                  <DetailField
                    label="Published on"
                    value={display.addedAt ? formatToLocalDate(display.addedAt) : 'Not available'}
                  />
                </dl>
              </section>
            </>
          )}
        </TabsContent>
        <TabsContent value="statistics" className="p-5">
          <AddonStatistics addon={display} />
        </TabsContent>
        <TabsContent value="history" className="p-5">
          {display.reviewHistory.length === 0 ? (
            <p className="text-sm text-muted-foreground">No submissions yet.</p>
          ) : (
            <div className="divide-y">
              {display.reviewHistory.map(entry => (
                <div
                  key={entry.submissionId}
                  className="flex flex-wrap items-center justify-between gap-2 py-3"
                >
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-xs tabular-nums text-muted-foreground">
                        #{entry.number}
                      </span>
                      <StatusChip tone={entry.tone} label={entry.status} />
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {entry.date ? formatToLocalDate(entry.date) : 'Not available'}
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => onSelect(`submission:${entry.submissionId}`)}
                  >
                    View review
                  </Button>
                </div>
              ))}
            </div>
          )}
        </TabsContent>
      </ScrollArea>
    </Tabs>
  )
}
