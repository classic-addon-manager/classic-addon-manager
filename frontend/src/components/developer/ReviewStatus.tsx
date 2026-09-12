import { Browser } from '@wailsio/runtime'
import { useState } from 'react'

import { backendUnavailable, type CatalogReview } from '@/components/developer/catalogEditing'
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
import { cn } from '@/lib/utils'

export function ReviewStatus({
  review,
  onInspect,
}: {
  review: CatalogReview
  onInspect: () => void
}) {
  const [withdrawing, setWithdrawing] = useState(false)
  const [dismissed, setDismissed] = useState(false)
  if (dismissed && review.status === 'approved') return null
  const labels = {
    in_review: ['Changes in review', 'Your published listing remains live until approval.'],
    approved: ['Changes approved and published', 'Your updated catalog details are now live.'],
    rejected: [
      'Catalog edits rejected',
      'Your addon is still published. None of these changes went live.',
    ],
    withdrawn: [
      'Changes withdrawn',
      'The review is closed. Future edits start from the published listing.',
    ],
  }
  return (
    <>
      <section
        className={cn(
          'space-y-3 rounded-xl border bg-primary/5 p-4',
          review.status === 'rejected' && 'border-destructive/30 bg-destructive/5',
          review.status === 'approved' && 'border-emerald-500/30 bg-emerald-500/5'
        )}
      >
        <div>
          <h3 className="text-sm font-medium">{labels[review.status][0]}</h3>
          <p className="mt-1 text-sm text-muted-foreground">{labels[review.status][1]}</p>
        </div>
        {review.status === 'rejected' && (
          <div className="border-l-2 border-destructive pl-3 text-sm">
            <p className="text-xs text-muted-foreground">Reviewer feedback</p>
            <p className="mt-1 whitespace-pre-wrap wrap-break-word">
              {review.feedback ?? 'Feedback is not available. Open the review for details.'}
            </p>
          </div>
        )}
        <div className="flex flex-wrap gap-2">
          {(review.status === 'in_review' || review.status === 'rejected') && (
            <Button variant="outline" size="sm" onClick={onInspect}>
              View {review.status === 'rejected' ? 'rejected' : 'submitted'} changes
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              // TODO(backend): Return the actual review URL with the review payload.
              if (review.htmlUrl.startsWith('https://github.com/'))
                void Browser.OpenURL(review.htmlUrl)
              else backendUnavailable('View review')
            }}
          >
            View review · #{review.prNumber}
          </Button>
          {review.status === 'in_review' && (
            <Button variant="ghost" size="sm" onClick={() => setWithdrawing(true)}>
              Withdraw changes
            </Button>
          )}
          {review.status === 'approved' && (
            <Button variant="ghost" size="sm" onClick={() => setDismissed(true)}>
              Dismiss
            </Button>
          )}
        </div>
      </section>
      <AlertDialog open={withdrawing} onOpenChange={setWithdrawing}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Withdraw these changes?</AlertDialogTitle>
            <AlertDialogDescription>
              This closes the review and removes the working proposal. The published addon and
              Submission history stay unchanged.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep in review</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={event => {
                event.preventDefault()
                // TODO(backend): Withdraw this review, check its latest revision/state, then refresh.
                // Do not remove the proposal on failure or claim success if approval won the race.
                backendUnavailable('Withdraw changes')
              }}
            >
              Withdraw changes
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}
