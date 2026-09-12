import { InfoIcon } from 'lucide-react'

import type { ReviewHistoryEntry, ReviewHistoryTone } from '@/components/developer/ownedParse'
import { toast } from '@/components/ui/toast'

/** Catalog-review outcome the addon's latest submission can be in. */
export type ReviewOutcome = 'in_review' | 'approved' | 'rejected' | 'withdrawn'

 // The addon's current catalog-review status, derived from its newest submission.
export type AddonReview = {
  /** Submission whose detail shows the real proposed declaration and feedback. */
  submissionId: number
  status: ReviewOutcome
}

/** Maps the shared review tones onto review outcomes, unknown stays unmapped. */
const REVIEW_OUTCOMES: Record<ReviewHistoryTone, ReviewOutcome | null> = {
  review: 'in_review',
  approved: 'approved',
  rejected: 'rejected',
  withdrawn: 'withdrawn',
  // An unmapped backend status must never be presented as a known outcome.
  unknown: null,
}

/**
 * Current review status from the addon's latest submission. History is
 * newest-first, an empty history or an unmapped status yields no review.
 */
export function latestReview(history: ReviewHistoryEntry[]): AddonReview | null {
  const latest = history[0]
  if (!latest) return null
  const status = REVIEW_OUTCOMES[latest.tone]
  if (!status) return null
  return {
    submissionId: latest.submissionId,
    status,
  }
}

export function backendUnavailable(action: string) {
  toast({
    title: action,
    description: 'Not implemented in the backend yet.',
    icon: InfoIcon,
  })
}
