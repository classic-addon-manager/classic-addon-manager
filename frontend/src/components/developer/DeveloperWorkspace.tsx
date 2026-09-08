import { Browser } from '@wailsio/runtime'
import { GithubIcon, Inbox } from 'lucide-react'
import { useState } from 'react'

import { AddonDetails, AddonIcon } from '@/components/developer/AddonDetails'
import { backendUnavailable } from '@/components/developer/catalogEditing'
import { publishFormFromPayload, type PublishFormState } from '@/components/developer/constants'
import type { OwnedSubmission } from '@/components/developer/ownedParse'
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
import { cn, formatToLocalDate, formatToLocalTime } from '@/lib/utils'

import type { OwnedAddonsData } from './useOwnedAddons'

export function DeveloperWorkspace({
  data,
  selection,
  onSelectionChange,
  onResubmit,
}: {
  data: OwnedAddonsData
  selection: string | null
  onSelectionChange: (key: string) => void
  onResubmit: (initial: PublishFormState) => void
}) {
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
  return (
    <div className="grid min-h-0 flex-1 grid-rows-[minmax(0,180px)_minmax(0,1fr)] md:grid-cols-[220px_minmax(0,1fr)] md:grid-rows-1">
      <ScrollArea className="min-h-0 border-b md:border-r md:border-b-0">
        <nav className="space-y-6 p-3" aria-label="Your published addons and submissions">
          {(['Published', 'Submissions'] as const).map(group => {
            const matching = entries.filter(entry =>
              group === 'Published' ? entry.addon : entry.submission
            )
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
          <AddonDetails key={selected.key} addon={selected.addon} />
        ) : (
          selected.submission && (
            <SubmissionDetails
              key={selected.key}
              submission={selected.submission}
              onSelectionChange={onSelectionChange}
              onResubmit={onResubmit}
            />
          )
        )}
      </section>
    </div>
  )
}

function SubmissionDetails({
  submission,
  onSelectionChange,
  onResubmit,
}: {
  submission: OwnedSubmission
  onSelectionChange: (key: string) => void
  onResubmit: (initial: PublishFormState) => void
}) {
  const [withdrawing, setWithdrawing] = useState(false)
  const open = submission.status === 'open'
  const hasMessages = submission.messages.length > 0
  return (
    <ScrollArea className="h-full">
      <div className="space-y-5 p-5">
        <div>
          <h2 className="break-words text-xl font-semibold tracking-tight">{submission.title}</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {open ? 'In review' : 'Changes requested'}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {submission.payload.repo !== '' && (
            <Button
              variant="outline"
              onClick={() => void Browser.OpenURL(`https://github.com/${submission.payload.repo}`)}
            >
              <GithubIcon />
              View code
            </Button>
          )}
          {open ? (
            <Button variant="outline" onClick={() => backendUnavailable('Revise submission')}>
              Revise submission
            </Button>
          ) : (
            <Button
              variant="outline"
              onClick={() => {
                onSelectionChange(`submission:${submission.id}`)
                onResubmit(publishFormFromPayload(submission.payload))
              }}
            >
              Resubmit for review
            </Button>
          )}
          {open && (
            <Button variant="ghost" onClick={() => setWithdrawing(true)}>
              Withdraw submission
            </Button>
          )}
        </div>
        <section
          className={cn(
            'space-y-3 rounded-xl border bg-primary/5 p-4',
            !open && 'border-destructive/30 bg-destructive/5'
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
          ) : (
            <>
              <div>
                <h3 className="text-sm font-medium">Changes requested</h3>
                <p className="mt-1 text-sm text-muted-foreground">
                  This addon is not published. None of these details are live.
                </p>
              </div>
              <div className="border-l-2 border-destructive pl-3 text-sm">
                <p className="text-xs text-muted-foreground">Reviewer feedback</p>
                {hasMessages ? (
                  <div className="mt-1 space-y-3">
                    {submission.messages.map(message => (
                      <div key={message.id}>
                        <p className="whitespace-pre-wrap break-words">{message.body}</p>
                        {message.createdAt !== null && (
                          <p className="mt-1 text-xs text-muted-foreground">
                            {formatToLocalTime(message.createdAt)}
                          </p>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="mt-1 whitespace-pre-wrap break-words">
                    Feedback is not available here.
                  </p>
                )}
              </div>
            </>
          )}
        </section>
        {open && hasMessages && (
          <section className="space-y-3">
            <h3 className="text-sm font-medium">Review</h3>
            {submission.messages.map(message => (
              <div key={message.id}>
                <p className="whitespace-pre-wrap break-words text-sm">{message.body}</p>
                {message.createdAt !== null && (
                  <p className="mt-1 text-xs text-muted-foreground">
                    {formatToLocalTime(message.createdAt)}
                  </p>
                )}
              </div>
            ))}
          </section>
        )}
        <section className="space-y-3">
          <h3 className="text-sm font-medium">Submitted declaration</h3>
          <p className="whitespace-pre-wrap break-words text-sm text-muted-foreground">
            {submission.payload.description || 'No description provided.'}
          </p>
          <dl className="divide-y rounded-xl border bg-card/40 px-4 text-sm">
            <DetailField label="Name" value={submission.payload.name} />
            <DetailField label="Alias" value={submission.payload.alias} />
            <DetailField label="Author" value={submission.payload.author} />
            <DetailField
              label="Repository"
              value={submission.payload.repo}
              href={
                submission.payload.repo !== ''
                  ? `https://github.com/${submission.payload.repo}`
                  : undefined
              }
            />
            <DetailField label="Branch" value={submission.payload.branch} />
            <DetailField label="Tags" value={submission.payload.tags.join(', ')} />
            <DetailField label="Keywords" value={submission.payload.keywords.join(', ')} />
            <DetailField label="Dependencies" value={submission.payload.dependencies.join(', ')} />
            <DetailField
              label="Ko-fi"
              value={submission.payload.kofi}
              href={
                submission.payload.kofi !== ''
                  ? `https://ko-fi.com/${submission.payload.kofi}`
                  : undefined
              }
            />
            <DetailField
              label="Submitted"
              value={
                submission.createdAt ? formatToLocalDate(submission.createdAt) : 'Not available'
              }
            />
          </dl>
        </section>
      </div>
      <AlertDialog open={withdrawing} onOpenChange={setWithdrawing}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Withdraw this submission?</AlertDialogTitle>
            <AlertDialogDescription>
              This closes the review and removes the working proposal. Review history is retained.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep in review</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={event => {
                event.preventDefault()
                // TODO(backend): Close the new-addon review, retaining history only after confirmation.
                backendUnavailable('Withdraw submission')
              }}
            >
              Withdraw submission
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </ScrollArea>
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
