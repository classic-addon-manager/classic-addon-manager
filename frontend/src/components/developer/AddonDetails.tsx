import { Browser } from '@wailsio/runtime'
import { BarChart3, BlocksIcon, Check, GithubIcon, LoaderCircle } from 'lucide-react'
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
  FORM_FIELD_ORDER,
  scrollFirstFieldErrorIntoView,
} from '@/components/developer/declarationFields'
import {
  mockCatalogReview,
  mockDownloadTrends,
  mockReviewHistory,
  type PreviewReviewState,
} from '@/components/developer/developerMocks'
import { valuesToForm } from '@/components/developer/formValues.ts'
import type { OwnedAddon } from '@/components/developer/ownedParse'
import { ReviewStatus } from '@/components/developer/ReviewStatus'
import { requireAddonSchema } from '@/components/developer/schema.ts'
import { useDevAddonValues } from '@/components/developer/useDevAddonValues.ts'
import { type FieldErrors, submitAddon, validateAddon } from '@/components/developer/validate'
import { getAddonValues } from '@/components/developer/values.ts'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { toast } from '@/components/ui/toast'
import { cn, formatToLocalDate } from '@/lib/utils'

export function AddonDetails({
  addon,
  onRefresh,
  onSelect,
}: {
  addon: OwnedAddon
  onRefresh: () => Promise<void>
  onSelect: (key: string) => void
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
  const [mode, setMode] = useState<'view' | 'edit' | 'compare'>('view')
  const [previewState, setPreviewState] = useState<PreviewReviewState>('none')
  const review = mockCatalogReview(display, previewState)
  const published = catalogFields(display)
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
                : 'Edit details'}
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
              initialCatalog={editableProposal}
              initialForm={form}
              submitLabel={submitLabel}
              onCancel={() => setMode('view')}
              onRefresh={onRefresh}
              onSelect={onSelect}
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
              {display.warning && (
                <p className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm">
                  {display.warning}
                </p>
              )}
              <section className="space-y-3">
                <h3 className="text-sm font-medium">Catalog details</h3>
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
                    label="Added to catalog"
                    value={display.addedAt ? formatToLocalDate(display.addedAt) : 'Not available'}
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
          <AddonStatistics addon={display} />
        </TabsContent>
      </ScrollArea>
    </Tabs>
  )
}
function AddonEditPanel({
  addon,
  initialCatalog,
  initialForm,
  submitLabel,
  onCancel,
  onRefresh,
  onSelect,
}: {
  addon: OwnedAddon
  initialCatalog: CatalogFields
  initialForm: PublishFormState | null
  submitLabel: string
  onCancel: () => void
  onRefresh: () => Promise<void>
  onSelect: (key: string) => void
}) {
  const initial =
    initialForm ??
    publishFormFromPayload({
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
  const [submissionId, setSubmissionId] = useState<number | null>(null)
  const [alreadyOpenId, setAlreadyOpenId] = useState<number | null>(null)
  const [validating, setValidating] = useState(false)
  const [validated, setValidated] = useState(false)
  const [saving, setSaving] = useState(false)
  const [editable, setEditable] = useState(true)
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})
  const [validationError, setValidationError] = useState(false)
  const [publishError, setPublishError] = useState<string | null>(null)
  const busy = validating || saving || !editable
  const dirty = isPublishFormDirty(form, initial)
  const hasFieldErrors = FORM_FIELD_ORDER.some(key => !!fieldErrors[key]?.length)
  const failCopy = hasFieldErrors
    ? 'Fix the highlighted fields.'
    : publishError !== null
      ? publishError
      : validationError
        ? "Couldn't validate this addon."
        : null
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
    setValidated(false)
    setValidationError(false)
    setPublishError(null)
    setFieldErrors(prev => {
      if (!prev[key]) return prev
      const next = { ...prev }
      delete next[key]
      return next
    })
  }

  const applyInvalidResult = (fields: FieldErrors) => {
    const hasFields = FORM_FIELD_ORDER.some(key => !!fields[key]?.length)
    if (hasFields) {
      setFieldErrors(fields)
      requestAnimationFrame(() => {
        scrollFirstFieldErrorIntoView(fields)
      })
      return
    }
    setValidationError(true)
  }

  const resumeExisting = async (id: number) => {
    const schema = await requireAddonSchema()
    if (!schema) {
      toast({ title: submitLabel, description: 'This source is unavailable.' })
      return
    }
    const result = await getAddonValues(
      { type: 'submission', id, kind: 'update', name: addon.name },
      schema
    )
    if (result.status !== 'ok') {
      toast({
        title: submitLabel,
        description:
          result.status === 'unauthorized' ||
          result.status === 'error' ||
          result.status === 'not_found'
            ? result.message
            : 'This source is unavailable.',
      })
      return
    }
    setSubmissionId(id)
    setForm(valuesToForm(result.values))
    setValidated(false)
    setValidationError(false)
    setFieldErrors({})
    setPublishError(null)
    setEditable(true)
  }

  const handleValidate = async () => {
    if (busy) return
    setValidating(true)
    setPublishError(null)
    setFieldErrors({})
    setValidationError(false)
    let validationPassed = false
    try {
      const result = await validateAddon(form, submissionId)
      if (result.status === 'not_open') {
        setEditable(false)
        setPublishError('This submission is no longer open.')
        return
      }
      if (result.status === 'error') {
        setValidationError(true)
        return
      }
      if (result.status === 'valid') {
        validationPassed = true
        return
      }
      applyInvalidResult(result.fields)
    } catch {
      setValidationError(true)
    } finally {
      setValidated(validationPassed)
      setValidating(false)
    }
  }

  const handleSubmit = async () => {
    if (busy || !validated) return
    setSaving(true)
    setPublishError(null)
    setFieldErrors({})
    try {
      const result = await submitAddon(form, submissionId)
      if (result.status === 'submitted') {
        toast({
          title: submissionId === null ? 'Addon submitted' : 'Submission updated',
          description: submissionId === null ? "It's now in review." : 'Your changes were saved.',
        })
        await onRefresh()
        onSelect(`submission:${result.id}`)
        return
      }
      if (result.status === 'already_open') {
        setAlreadyOpenId(result.id)
        return
      }
      if (result.status === 'invalid') {
        setValidated(false)
        applyInvalidResult(result.fields)
        return
      }
      if (result.status === 'not_open') {
        setEditable(false)
        setPublishError('This submission is no longer open.')
        return
      }
      setPublishError('message' in result ? result.message : "Couldn't publish this addon.")
    } catch {
      setPublishError("Couldn't publish this addon.")
    } finally {
      setSaving(false)
    }
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
        fieldErrors={fieldErrors}
        busy={busy}
        lockedFields={['name']}
      />
      {failCopy && <p className="text-sm text-destructive">{failCopy}</p>}
      <div className="flex flex-wrap gap-2 border-t pt-4">
        <div className="relative">
          {validated ? (
            <span
              className="publish-cta-ring pointer-events-none absolute inset-0 rounded-md"
              aria-hidden
            />
          ) : null}
          <Button
            type="button"
            disabled={busy || (!validated && !dirty)}
            onClick={() => {
              if (validated) {
                void handleSubmit()
                return
              }
              void handleValidate()
            }}
            className={cn(
              validated &&
                'bg-emerald-600 text-emerald-50 hover:bg-emerald-700 focus-visible:border-emerald-600 focus-visible:ring-emerald-400/50 publish-cta-scale'
            )}
          >
            {saving ? (
              <>
                <LoaderCircle className="animate-spin" />
                {submitLabel}
              </>
            ) : validating ? (
              <>
                <LoaderCircle className="animate-spin" />
                Validate
              </>
            ) : validated ? (
              <>
                <Check />
                {submitLabel}
              </>
            ) : (
              'Validate'
            )}
          </Button>
        </div>
        <Button type="button" variant="outline" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
      </div>
      <AlertDialog
        open={alreadyOpenId !== null}
        onOpenChange={open => {
          if (!open) setAlreadyOpenId(null)
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Submission already open</AlertDialogTitle>
            <AlertDialogDescription>
              You already have an open submission for this name.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setAlreadyOpenId(null)}>
              Keep editing
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                const id = alreadyOpenId
                setAlreadyOpenId(null)
                if (id === null) return
                void resumeExisting(id)
              }}
            >
              Resume existing
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

function DetailField({ label, value, href }: { label: string; value: string; href?: string }) {
  return (
    <div className="flex flex-wrap justify-between gap-x-4 gap-y-1 py-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 wrap-break-word">
        {value && href ? (
          <button
            type="button"
            className="cursor-pointer text-left wrap-break-word text-primary underline-offset-4 hover:underline"
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
