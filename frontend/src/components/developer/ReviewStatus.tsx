import { AlertTriangleIcon, CheckIcon } from 'lucide-react'
import { useState } from 'react'

import type { AddonReview } from '@/components/developer/catalogEditing'
import { withdrawDeclaration } from '@/components/developer/declarationApi.ts'
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
import { toast } from '@/components/ui/toast'
import { cn } from '@/lib/utils'

export function ReviewStatus({
  review,
  onInspect,
  onRefresh,
}: {
  review: AddonReview
  onInspect: () => void
  onRefresh: () => Promise<void>
}) {
  const [withdrawing, setWithdrawing] = useState(false)
  const [dismissed, setDismissed] = useState(false)
  const [busy, setBusy] = useState(false)
  // The backend can close a review underneath us, stop offering withdrawal once it does.
  const [closed, setClosed] = useState(false)
  if (dismissed && review.status === 'approved') return null
  const labels = {
    in_review: ['Changes in review', 'Your published listing remains live until approval.'],
    approved: ['Changes approved and published', 'Your updated listing is now live.'],
    rejected: [
      'Listing edits rejected',
      'Your addon is still published. None of these changes went live.',
    ],
    withdrawn: [
      'Changes withdrawn',
      'The review is closed. Future edits start from the published listing.',
    ],
  }

  const handleWithdraw = async () => {
    if (busy) return
    setBusy(true)
    try {
      const result = await withdrawDeclaration(review.submissionId)
      if (result.status === 'withdrawn') {
        setWithdrawing(false)
        setClosed(true)
        toast({
          title: 'Changes withdrawn',
          description: 'The review is closed.',
          icon: CheckIcon,
        })
        void onRefresh()
        return
      }
      if (result.status === 'not_open') {
        setWithdrawing(false)
        setClosed(true)
        toast({
          title: 'Withdraw changes',
          description: 'This review is no longer open.',
          icon: AlertTriangleIcon,
        })
        void onRefresh()
        return
      }
      if (result.status === 'not_found') {
        setWithdrawing(false)
        toast({
          title: 'Withdraw changes',
          description: result.message,
          icon: AlertTriangleIcon,
        })
        void onRefresh()
        return
      }
      toast({
        title: 'Withdraw changes',
        description: result.message,
        icon: AlertTriangleIcon,
      })
    } finally {
      setBusy(false)
    }
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
        <div className="flex flex-wrap gap-2">
          {(review.status === 'in_review' || review.status === 'rejected') && (
            <Button variant="outline" size="sm" onClick={onInspect}>
              View {review.status === 'rejected' ? 'rejected' : 'submitted'} changes
            </Button>
          )}
          {review.status === 'in_review' && !closed && (
            <Button variant="ghost" size="sm" onClick={() => setWithdrawing(true)} disabled={busy}>
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
      <AlertDialog
        open={withdrawing}
        onOpenChange={next => {
          if (!busy) setWithdrawing(next)
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Withdraw these changes?</AlertDialogTitle>
            <AlertDialogDescription>
              This closes the review and removes the working proposal. The published addon and
              Submission history stay unchanged.
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
              {busy ? 'Withdrawing...' : 'Withdraw changes'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}
