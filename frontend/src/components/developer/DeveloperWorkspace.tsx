import { Browser } from '@wailsio/runtime'
import { GithubIcon, GitPullRequest } from 'lucide-react'
import { useState } from 'react'

import { AddonDetails, AddonIcon } from '@/components/developer/AddonDetails'
import { backendUnavailable } from '@/components/developer/catalogEditing'
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
import { cn, formatToLocalDate } from '@/lib/utils'

import type { OwnedAddonsData } from './useOwnedAddons'

export function DeveloperWorkspace({ data }: { data: OwnedAddonsData }) {
  const [selection, setSelection] = useState<string | null>(null)
  const entries = [
    ...data.addons.map(addon => ({ key: `addon:${addon.name}`, addon, submission: null })),
    ...data.submissions.map(submission => ({
      key: `submission:${submission.prNumber}`,
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
                      onClick={() => setSelection(entry.key)}
                      className={cn(
                        'flex w-full items-center gap-3 rounded-lg border border-transparent px-2 py-3 text-left transition-colors hover:bg-muted/50 focus-visible:outline-2 focus-visible:outline-ring',
                        selected.key === entry.key && 'border-primary/20 bg-primary/10'
                      )}
                    >
                      {entry.addon ? (
                        <AddonIcon addon={entry.addon} />
                      ) : (
                        <GitPullRequest className="size-5 shrink-0 text-muted-foreground" />
                      )}
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium">
                          {entry.addon?.alias ?? entry.submission?.title}
                        </p>
                        <p
                          className={cn(
                            'mt-1 text-xs text-muted-foreground',
                            entry.submission?.status === 'open' && 'text-primary',
                            entry.submission?.status === 'closed' && 'text-destructive'
                          )}
                        >
                          {entry.addon
                            ? `${entry.addon.downloads.toLocaleString()} downloads`
                            : entry.submission?.status === 'open'
                              ? 'In review'
                              : 'Rejected'}
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
            <SubmissionDetails key={selected.key} submission={selected.submission} />
          )
        )}
      </section>
    </div>
  )
}

function SubmissionDetails({ submission }: { submission: OwnedSubmission }) {
  const [withdrawing, setWithdrawing] = useState(false)
  return (
    <ScrollArea className="h-full">
      <div className="space-y-5 p-5">
        <div>
          <h2 className="break-words text-xl font-semibold tracking-tight">{submission.title}</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {submission.status === 'open' ? 'In review' : 'Rejected'} · #{submission.prNumber}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {submission.htmlUrl.startsWith('https://github.com/') && (
            <Button onClick={() => void Browser.OpenURL(submission.htmlUrl)}>
              <GitPullRequest />
              View PR
            </Button>
          )}
          {submission.repo && (
            <Button
              variant="outline"
              onClick={() => void Browser.OpenURL(`https://github.com/${submission.repo}`)}
            >
              <GithubIcon />
              View code
            </Button>
          )}
          {/* TODO(backend): Fetch the submitted payload and reopen/revise the same review thread. */}
          <Button
            variant="outline"
            onClick={() =>
              backendUnavailable(
                submission.status === 'open' ? 'Revise submission' : 'Resubmit for review'
              )
            }
          >
            {submission.status === 'open' ? 'Revise submission' : 'Resubmit for review'}
          </Button>
          {submission.status === 'open' && (
            <Button variant="ghost" onClick={() => setWithdrawing(true)}>
              Withdraw submission
            </Button>
          )}
        </div>
        <section
          className={cn(
            'rounded-xl border bg-primary/5 p-4',
            submission.status === 'closed' && 'border-destructive/30 bg-destructive/5'
          )}
        >
          <h3 className="text-sm font-medium">
            {submission.status === 'open' ? 'Awaiting review' : 'Submission rejected'}
          </h3>
          <p className="mt-1 text-sm text-muted-foreground">
            {submission.status === 'open'
              ? 'Follow the pull request for review updates.'
              : 'Open the pull request to read the feedback before resubmitting.'}
          </p>
        </section>
        <dl className="space-y-3 text-sm">
          <div>
            <dt className="text-muted-foreground">Repository</dt>
            <dd className="break-words">{submission.repo ?? 'Not available'}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Submitted</dt>
            <dd>
              {submission.createdAt ? formatToLocalDate(submission.createdAt) : 'Not available'}
            </dd>
          </div>
        </dl>
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
