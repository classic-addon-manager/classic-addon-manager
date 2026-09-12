import { Browser } from '@wailsio/runtime'
import {
  AlertTriangleIcon,
  ArrowLeft,
  CheckIcon,
  GithubIcon,
  Inbox,
  LoaderCircle,
} from 'lucide-react'
import { useState } from 'react'

import { AddonDetails, AddonEditPanel, AddonIcon } from '@/components/developer/AddonDetails'
import { publishFormFromPayload } from '@/components/developer/constants'
import { withdrawDeclaration } from '@/components/developer/declarationApi.ts'
import { valuesToForm } from '@/components/developer/formValues.ts'
import { statusTone } from '@/components/developer/ownedAddons'
import type { OwnedSubmission } from '@/components/developer/ownedParse'
import { StatusChip } from '@/components/developer/StatusChip'
import type { SubmissionStatus } from '@/components/developer/types.ts'
import { useDevAddonSubmission } from '@/components/developer/useDevAddonSubmission.ts'
import { useDevAddonValues } from '@/components/developer/useDevAddonValues.ts'
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
import { toast } from '@/components/ui/toast'
import { cn, formatToLocalDate, formatToLocalTime } from '@/lib/utils'

import type { OwnedAddonsData } from './useOwnedAddons'

/** Review-view wording per submission status, paired with the shared status tones. */
const SUBMISSION_STATUS_LABELS: Record<SubmissionStatus, string> = {
  open: 'In review',
  approved: 'Approved',
  rejected: 'Changes requested',
  withdrawn: 'Withdrawn',
}

export function DeveloperWorkspace({
  data,
  selection,
  onSelectionChange,
  onRefresh,
  onDropSubmission,
}: {
  data: OwnedAddonsData
  selection: string | null
  onSelectionChange: (key: string) => void
  onRefresh: () => Promise<void>
  onDropSubmission: (id: number) => void
}) {
  const publishedNames = new Set(data.addons.map(addon => addon.name))
  const entries = [
    ...data.addons.map(addon => ({ key: `addon:${addon.name}`, addon, submission: null })),
    ...data.submissions.map(submission => ({
      key: `submission:${submission.id}`,
      addon: null,
      submission,
    })),
  ]
  // Polling may remove a submission after publication; fall back without retaining an old form.
  const selected = entries.find(entry => entry.key === selection) ?? entries[0]
  if (!selected) return null
  // A submission reached from an addon's review history returns to that addon.
  const backToAddon = selected.submission
    ? data.addons.find(addon => addon.name === selected.submission?.payload.name)
    : undefined
  return (
    <div className="grid min-h-0 flex-1 grid-rows-[minmax(0,180px)_minmax(0,1fr)] md:grid-cols-[220px_minmax(0,1fr)] md:grid-rows-1">
      <ScrollArea className="min-h-0 border-b md:border-r md:border-b-0">
        <nav className="space-y-6 p-3" aria-label="Your published addons and submissions">
          {(['Published', 'Submissions'] as const).map(group => {
            const matching = entries.filter(entry => {
              if (group === 'Published') return entry.addon
              return (
                entry.submission &&
                !(
                  entry.submission.kind === 'update' &&
                  publishedNames.has(entry.submission.payload.name)
                )
              )
            })
            if (!matching.length) return null
            return (
              <section key={group}>
                <h2 className="mb-2 flex justify-between px-2 text-xs font-medium text-muted-foreground">
                  {group}
                  <span>{matching.length}</span>
                </h2>
                <div className="space-y-1">
                  {matching.map(entry => (
                    <button
                      key={entry.key}
                      type="button"
                      aria-pressed={selected.key === entry.key}
                      onClick={() => onSelectionChange(entry.key)}
                      className={cn(
                        'flex w-full items-center gap-3 rounded-lg border border-transparent px-2 py-3 text-left transition-colors hover:bg-muted/50 focus-visible:outline-2 focus-visible:outline-ring',
                        selected.key === entry.key && 'border-primary/20 bg-primary/10'
                      )}
                    >
                      {entry.addon ? (
                        <AddonIcon addon={entry.addon} />
                      ) : (
                        <Inbox className="size-5 shrink-0 text-muted-foreground" />
                      )}
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium">
                          {entry.addon?.alias ?? entry.submission?.title}
                        </p>
                        <p
                          className={cn(
                            'mt-1 text-xs text-muted-foreground',
                            entry.submission?.status === 'open' && 'text-primary',
                            entry.submission?.status === 'rejected' && 'text-destructive'
                          )}
                        >
                          {entry.addon
                            ? `${entry.addon.downloads.toLocaleString()} downloads`
                            : entry.submission?.status === 'open'
                              ? 'In review'
                              : 'Changes requested'}
                        </p>
                      </div>
                    </button>
                  ))}
                </div>
              </section>
            )
          })}
        </nav>
      </ScrollArea>
      <section className="min-h-0 min-w-0" aria-label="Selected addon details">
        {selected.addon ? (
          <AddonDetails
            key={selected.key}
            addon={selected.addon}
            onSelect={onSelectionChange}
            onRefresh={onRefresh}
          />
        ) : (
          selected.submission && (
            <SubmissionDetails
              key={selected.key}
              submission={selected.submission}
              backTo={
                backToAddon
                  ? {
                      key: `addon:${backToAddon.name}`,
                      label: backToAddon.alias || backToAddon.name,
                    }
                  : undefined
              }
              onSelectionChange={onSelectionChange}
              onRefresh={onRefresh}
              onDropSubmission={onDropSubmission}
            />
          )
        )}
      </section>
    </div>
  )
}

function SubmissionDetails({
  submission,
  backTo,
  onSelectionChange,
  onRefresh,
  onDropSubmission,
}: {
  submission: OwnedSubmission
  backTo?: { key: string; label: string }
  onSelectionChange: (key: string) => void
  onRefresh: () => Promise<void>
  onDropSubmission: (id: number) => void
}) {
  const loaded = useDevAddonValues({
    type: 'submission',
    id: submission.id,
    kind: 'new',
    name: submission.title,
  })
  // Review metadata comes from the submission endpoint; the list payload only
  // seeds the sidebar and is not authoritative for status, kind, or comments.
  const detail = useDevAddonSubmission(submission.id)
  const submissionDetail = detail.submission
  const form = loaded.values ? valuesToForm(loaded.values) : null
  const payload = form
    ? {
        name: form.name,
        alias: form.alias,
        description: form.description,
        author: form.author,
        repo: form.repo,
        branch: form.branch,
        tags: [...form.tags],
        keywords: [...form.keywords],
        dependencies: [...form.dependencies],
        kofi: form.kofi,
      }
    : submission.payload
  const [withdrawing, setWithdrawing] = useState(false)
  const [editing, setEditing] = useState(false)
  const [busy, setBusy] = useState(false)
  const [closed, setClosed] = useState(false)
  const status = submissionDetail?.status ?? submission.status
  const kind = submissionDetail?.kind ?? submission.kind
  const createdAt = submissionDetail?.createdAt ?? submission.createdAt
  const messages = submissionDetail?.messages ?? submission.messages
  const round = Math.max(1, submissionDetail?.revision ?? 1)
  const relevantMessages = messages.filter(
    message => Math.max(1, message.revision) === round
  )
  const hasMessages = relevantMessages.length > 0
  const open = status === 'open'
  const rejected = status === 'rejected'
  const editable = open && !closed
  const fromHistory = backTo !== undefined

  const handleWithdraw = async () => {
    if (busy) return
    setBusy(true)
    try {
      const result = await withdrawDeclaration(submission.id)
      if (result.status === 'withdrawn') {
        setWithdrawing(false)
        toast({
          title: 'Submission withdrawn',
          description: 'The review is closed.',
          icon: CheckIcon,
        })
        onDropSubmission(submission.id)
        void onRefresh()
        return
      }
      if (result.status === 'not_open') {
        setWithdrawing(false)
        setClosed(true)
        toast({
          title: 'Withdraw submission',
          description: 'This submission is no longer open.',
          icon: AlertTriangleIcon,
        })
        void onRefresh()
        return
      }
      if (result.status === 'not_found') {
        setWithdrawing(false)
        toast({
          title: 'Withdraw submission',
          description: result.message,
          icon: AlertTriangleIcon,
        })
        onDropSubmission(submission.id)
        void onRefresh()
        return
      }
      toast({
        title: 'Withdraw submission',
        description: result.message,
        icon: AlertTriangleIcon,
      })
    } finally {
      setBusy(false)
    }
  }
  if (loaded.loading || detail.loading) {
    return (
      <div className="flex h-full flex-col items-center justify-center text-muted-foreground">
        <LoaderCircle className="size-8 animate-spin opacity-50" strokeWidth={1.5} />
        <p className="mt-3 text-sm">Loading declaration...</p>
      </div>
    )
  }
  const loadError = loaded.error ?? detail.error
  if (loadError) {
    return (
      <div className="flex h-full items-center justify-center p-5 text-sm text-destructive">
        {loadError}
      </div>
    )
  }
  return (
    <ScrollArea className="h-full">
      <div className="space-y-5 p-5">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h2 className="wrap-break-word text-xl font-semibold tracking-tight">
              {payload.alias || payload.name}
            </h2>
            <div className="mt-1">
              {closed ? (
                // The backend closed this review, its outcome is unknown here.
                <StatusChip tone="unknown" label="No longer open" />
              ) : (
                <StatusChip tone={statusTone(status)} label={SUBMISSION_STATUS_LABELS[status]} />
              )}
            </div>
          </div>
          {backTo && (
            <Button
              variant="ghost"
              size="sm"
              className="shrink-0"
              onClick={() => onSelectionChange(backTo.key)}
            >
              <ArrowLeft />
              Back to {backTo.label}
            </Button>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          {payload.repo !== '' && !fromHistory && (
            <Button
              variant="outline"
              onClick={() => void Browser.OpenURL(`https://github.com/${payload.repo}`)}
            >
              <GithubIcon />
              View code
            </Button>
          )}
          {editing ? null : editable ? (
            <Button variant="outline" onClick={() => setEditing(true)}>
              Revise submission
            </Button>
          ) : (
            rejected &&
            !closed && (
              <Button variant="outline" onClick={() => setEditing(true)}>
                Resubmit for review
              </Button>
            )
          )}
          {editable && (
            <Button variant="ghost" onClick={() => setWithdrawing(true)} disabled={busy}>
              Withdraw submission
            </Button>
          )}
        </div>
        <section
          className={cn(
            'space-y-3 rounded-xl border bg-primary/5 p-4',
            rejected && 'border-destructive/30 bg-destructive/5'
          )}
        >
          {open ? (
            <>
              <h3 className="text-sm font-medium">
                {hasMessages ? 'In review' : 'Awaiting review'}
              </h3>
              {!hasMessages && (
                <p className="mt-1 text-sm text-muted-foreground">
                  Reviewers will leave feedback here.
                </p>
              )}
            </>
          ) : rejected ? (
            <>
              <div>
                <h3 className="text-sm font-medium">Changes requested</h3>
                <p className="mt-1 text-sm text-muted-foreground">
                  None of these changes are live.
                </p>
              </div>
              <div className="border-l-2 border-destructive pl-3 text-sm">
                <p className="text-xs text-muted-foreground">Reviewer feedback</p>
                {hasMessages ? (
                  <div className="mt-1 space-y-3">
                    {relevantMessages.map(message => (
                      <div key={message.id}>
                        <p className="whitespace-pre-wrap wrap-break-word">{message.body}</p>
                        {message.createdAt !== null && (
                          <p className="mt-1 text-xs text-muted-foreground">
                            {formatToLocalTime(message.createdAt)}
                          </p>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="mt-1 whitespace-pre-wrap wrap-break-word">
                    Feedback is not available.
                  </p>
                )}
              </div>
            </>
          ) : (
            <>
              <h3 className="text-sm font-medium">
                {status === 'approved' ? 'Approved' : 'Withdrawn'}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {status === 'approved'
                  ? 'This submission was approved and is now live in the app.'
                  : 'This review is closed and no longer actionable.'}
              </p>
            </>
          )}
        </section>
        {!rejected && hasMessages && (
          <section className="space-y-3">
            <h3 className="text-sm font-medium">Review</h3>
            {relevantMessages.map(message => (
              <div key={message.id}>
                <p className="whitespace-pre-wrap wrap-break-word text-sm">{message.body}</p>
                {message.createdAt !== null && (
                  <p className="mt-1 text-xs text-muted-foreground">
                    {formatToLocalTime(message.createdAt)}
                  </p>
                )}
              </div>
            ))}
          </section>
        )}
        {editing ? (
          <AddonEditPanel
            addon={{
              ...submission.payload,
              uuid: '',
              downloads: 0,
              likePercentage: null,
              warning: null,
              addedAt: null,
              reviewHistory: [],
            }}
            initialForm={publishFormFromPayload(payload)}
            initialSubmissionId={rejected ? null : submission.id}
            nameLocked={kind === 'update'}
            requireChanges={false}
            introTitle={rejected ? 'Resubmit for review' : 'Update submission'}
            introNote="Leaving this form discards unsent edits. Nothing goes live until this submission is approved."
            submitLabel={rejected ? 'Resubmit for review' : 'Update submission'}
            onCancel={() => {
              setEditing(false)
              void onRefresh()
            }}
          />
        ) : (
          <section className="space-y-3">
            <h3 className="text-sm font-medium">Submitted declaration</h3>
            <p className="whitespace-pre-wrap wrap-break-word text-sm text-muted-foreground">
              {payload.description || 'No description provided.'}
            </p>
            <dl className="divide-y rounded-xl border bg-card/40 px-4 text-sm">
              <DetailField label="Name" value={payload.name} />
              <DetailField label="Alias" value={payload.alias} />
              <DetailField label="Author" value={payload.author} />
              <DetailField
                label="Repository"
                value={payload.repo}
                href={payload.repo !== '' ? `https://github.com/${payload.repo}` : undefined}
              />
              <DetailField label="Branch" value={payload.branch} />
              <DetailField label="Tags" value={payload.tags.join(', ')} />
              <DetailField label="Keywords" value={payload.keywords.join(', ')} />
              <DetailField label="Dependencies" value={payload.dependencies.join(', ')} />
              <DetailField
                label="Ko-fi"
                value={payload.kofi}
                href={payload.kofi !== '' ? `https://ko-fi.com/${payload.kofi}` : undefined}
              />
              <DetailField
                label="Submitted"
                value={createdAt ? formatToLocalDate(createdAt) : 'Not available'}
              />
            </dl>
          </section>
        )}
        <AlertDialog
          open={withdrawing}
          onOpenChange={next => {
            if (!busy) setWithdrawing(next)
          }}
        >
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Withdraw this submission?</AlertDialogTitle>
              <AlertDialogDescription>
                This closes the review and removes the working proposal. Submission history is
                retained.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={busy}>Keep in review</AlertDialogCancel>
              <AlertDialogAction
                variant="destructive"
                disabled={busy}
                onClick={event => {
                  event.preventDefault()
                  void handleWithdraw()
                }}
              >
                {busy ? 'Withdrawing...' : 'Withdraw submission'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </ScrollArea>
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
