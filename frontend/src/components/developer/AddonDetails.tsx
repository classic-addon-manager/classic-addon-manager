import { Browser } from '@wailsio/runtime'
import { BarChart3, BlocksIcon, GithubIcon } from 'lucide-react'
import { useState } from 'react'

import {
  AddonDeclarationFields,
  type SetDeclarationField,
} from '@/components/developer/AddonDeclarationFields'
import { CatalogComparison } from '@/components/developer/CatalogEditForm'
import {
  backendUnavailable,
  type CatalogFields,
  catalogFields,
} from '@/components/developer/catalogEditing'
import {
  isPublishFormDirty,
  publishFormFromPayload,
  type PublishFormState,
} from '@/components/developer/constants'
import {
  mockCatalogReview,
  mockDownloadTrends,
  mockReviewHistory,
  type PreviewReviewState,
} from '@/components/developer/developerMocks'
import type { OwnedAddon } from '@/components/developer/ownedParse'
import { ReviewStatus } from '@/components/developer/ReviewStatus'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { formatToLocalDate } from '@/lib/utils'

export function AddonDetails({ addon }: { addon: OwnedAddon }) {
  const [tab, setTab] = useState('overview')
  const [mode, setMode] = useState<'view' | 'edit' | 'compare'>('view')
  const [previewState, setPreviewState] = useState<PreviewReviewState>('none')
  const review = mockCatalogReview(addon, previewState)
  const published = catalogFields(addon)
  const editableProposal =
    review?.status === 'in_review' || review?.status === 'rejected' ? review.proposed : published
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
          <AddonIcon addon={addon} />
          <div className="min-w-0">
            <h2 className="break-words text-xl font-semibold tracking-tight">{addon.alias}</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              Published{addon.author && ` · ${addon.author}`}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button onClick={edit}>
            {review?.status === 'rejected'
              ? 'Revise changes'
              : review?.status === 'in_review'
                ? 'Edit pending changes'
                : 'Edit details'}
          </Button>
          {addon.repo && (
            <Button
              variant="outline"
              onClick={() => void Browser.OpenURL(`https://github.com/${addon.repo}`)}
            >
              <GithubIcon />
              View code
            </Button>
          )}
        </div>
        {/* TODO(backend): Remove the sample selector when real review state is available. */}
        <label className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          Sample review state
          <select
            aria-label="Sample review state"
            className="max-w-full rounded-md border bg-background px-2 py-1.5 text-foreground"
            value={previewState}
            onChange={event => {
              setMode('view')
              setPreviewState(event.target.value as PreviewReviewState)
            }}
          >
            <option value="none">No pending edits</option>
            <option value="in_review">In review</option>
            <option value="approved">Approved</option>
            <option value="rejected">Rejected</option>
            <option value="withdrawn">Withdrawn</option>
          </select>
        </label>
        <TabsList className="h-auto rounded-none border-b bg-transparent p-0">
          <TabsTrigger value="overview" className="rounded-none px-3 py-2">
            Overview
          </TabsTrigger>
          <TabsTrigger value="statistics" className="rounded-none px-3 py-2">
            Statistics
          </TabsTrigger>
        </TabsList>
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <TabsContent value="overview" className="space-y-5 p-5">
          {mode === 'edit' ? (
            <AddonEditPanel
              addon={addon}
              initialCatalog={editableProposal}
              submitLabel={submitLabel}
              onCancel={() => setMode('view')}
            />
          ) : mode === 'compare' && review ? (
            <>
              <h3 className="font-medium">
                {review.status === 'rejected' ? 'Rejected changes' : 'Submitted changes'}{' '}
                <span className="text-xs text-muted-foreground">· Sample data</span>
              </h3>
              <CatalogComparison published={published} proposed={review.proposed} />
              <div className="flex flex-wrap gap-2">
                <Button onClick={edit}>Revise changes</Button>
                <Button variant="outline" onClick={() => setMode('view')}>
                  Back to overview
                </Button>
              </div>
            </>
          ) : (
            <>
              {review && (
                <ReviewStatus
                  key={previewState}
                  review={review}
                  onInspect={() => setMode('compare')}
                />
              )}
              {addon.warning && (
                <p className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm">
                  {addon.warning}
                </p>
              )}
              <section className="space-y-3">
                <h3 className="text-sm font-medium">Catalog details</h3>
                <p className="whitespace-pre-wrap break-words text-sm text-muted-foreground">
                  {addon.description || 'No description provided.'}
                </p>
                <dl className="divide-y rounded-xl border bg-card/40 px-4 text-sm">
                  <DetailField
                    label="Repository"
                    value={addon.repo}
                    href={addon.repo !== '' ? `https://github.com/${addon.repo}` : undefined}
                  />
                  <DetailField label="Branch" value={addon.branch ?? 'Not specified'} />
                  <DetailField label="Tags" value={addon.tags.join(', ')} />
                  <DetailField
                    label="Added to catalog"
                    value={addon.addedAt ? formatToLocalDate(addon.addedAt) : 'Not available'}
                  />
                </dl>
              </section>
              <details
                className="border-t pt-4 text-sm"
                onToggle={event => {
                  if (event.currentTarget.open) backendUnavailable('Review history')
                }}
              >
                <summary className="cursor-pointer font-medium">
                  Review history{' '}
                  <span className="text-xs font-normal text-muted-foreground">· Sample data</span>
                </summary>
                {/* TODO(backend): Load all review outcomes, including closed withdrawn threads. */}
                <div className="mt-3 divide-y">
                  {mockReviewHistory.map(entry => (
                    <div
                      key={entry.number}
                      className="flex flex-wrap items-center justify-between gap-2 py-3"
                    >
                      <div>
                        #{entry.number} · {entry.status}
                        <p className="text-xs text-muted-foreground">{entry.date}</p>
                      </div>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => backendUnavailable('Review history')}
                      >
                        View review
                      </Button>
                    </div>
                  ))}
                </div>
              </details>
            </>
          )}
        </TabsContent>
        <TabsContent value="statistics" className="p-5">
          <AddonStatistics addon={addon} />
        </TabsContent>
      </ScrollArea>
    </Tabs>
  )
}

function AddonEditPanel({
  addon,
  initialCatalog,
  submitLabel,
  onCancel,
}: {
  addon: OwnedAddon
  initialCatalog: CatalogFields
  submitLabel: string
  onCancel: () => void
}) {
  const initial = publishFormFromPayload({
    name: addon.name,
    alias: initialCatalog.alias,
    description: initialCatalog.description,
    author: addon.author,
    repo: initialCatalog.repo,
    branch: initialCatalog.branch,
    tags: initialCatalog.tags,
    keywords: [],
    dependencies: [],
    kofi: '',
  })
  const [form, setForm] = useState<PublishFormState>(initial)
  const setField: SetDeclarationField = (key, value) => {
    setForm(prev => {
      const nextValue =
        typeof value === 'function'
          ? (value as (current: PublishFormState[typeof key]) => PublishFormState[typeof key])(
              prev[key]
            )
          : value
      if (Object.is(nextValue, prev[key])) return prev
      return { ...prev, [key]: nextValue }
    })
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="font-medium">Edit catalog details</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          Leaving this form discards unsent edits. Your published listing stays unchanged until
          approval.
        </p>
      </div>
      <AddonDeclarationFields
        form={form}
        setField={setField}
        fieldErrors={{}}
        busy={false}
        lockedFields={['name']}
      />
      <div className="flex flex-wrap gap-2 border-t pt-4">
        <Button
          type="button"
          disabled={!isPublishFormDirty(form, initial)}
          onClick={() => {
            // TODO(backend): Submit, update, or reopen the versioned catalog review here.
            // Rejected edits reopen the same thread. Never change published values optimistically.
            backendUnavailable(submitLabel)
          }}
        >
          {submitLabel}
        </Button>
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  )
}

function DetailField({ label, value, href }: { label: string; value: string; href?: string }) {
  return (
    <div className="flex flex-wrap justify-between gap-x-4 gap-y-1 py-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words">
        {value && href ? (
          <button
            type="button"
            className="cursor-pointer text-left break-words text-primary underline-offset-4 hover:underline"
            onClick={() => void Browser.OpenURL(href)}
          >
            {value}
          </button>
        ) : (
          value || 'None'
        )}
      </dd>
    </div>
  )
}

export function AddonIcon({ addon }: { addon: OwnedAddon }) {
  const [failed, setFailed] = useState(false)
  return addon.branch && !failed ? (
    <img
      className="size-10 shrink-0 rounded-lg border object-cover"
      src={`https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`}
      alt=""
      onError={() => setFailed(true)}
    />
  ) : (
    <div className="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-card text-primary">
      <BlocksIcon className="size-5" />
    </div>
  )
}

function AddonStatistics({ addon }: { addon: OwnedAddon }) {
  const [period, setPeriod] = useState<'7' | '30'>('7')
  const points = mockDownloadTrends[period]
  const maximum = Math.max(...points)
  return (
    <section className="space-y-6">
      <div className="flex flex-wrap gap-8 border-b pb-5">
        <div>
          <p className="text-xs text-muted-foreground">Total downloads</p>
          <p className="mt-1 text-2xl tabular-nums">{addon.downloads.toLocaleString()}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Likes</p>
          <p className="mt-1 text-2xl tabular-nums">
            {addon.likePercentage === null ? 'Not available' : `${addon.likePercentage}%`}
          </p>
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h3 className="flex items-center gap-2 text-sm font-medium">
          <BarChart3 className="size-4" />
          Download trend
        </h3>
        <div className="flex gap-1">
          {(['7', '30'] as const).map(value => (
            <Button
              key={value}
              size="sm"
              variant={period === value ? 'secondary' : 'ghost'}
              aria-pressed={period === value}
              onClick={() => {
                setPeriod(value)
                backendUnavailable('Download history')
              }}
            >
              {value} days
            </Button>
          ))}
        </div>
      </div>
      <p className="text-xs text-muted-foreground">
        Sample trend data. Totals above are from your published addon.
      </p>
      <div
        className="flex h-40 items-end gap-2 border-b"
        role="img"
        aria-label={`Sample download counts: ${points.join(', ')}`}
      >
        {points.map((point, index) => (
          <div
            key={index}
            className="min-w-0 flex-1 rounded-t bg-primary/80"
            style={{ height: `${(point / maximum) * 100}%` }}
            title={`${point} sample downloads`}
          />
        ))}
      </div>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{period} days ago</span>
        <span>Today</span>
      </div>
    </section>
  )
}
