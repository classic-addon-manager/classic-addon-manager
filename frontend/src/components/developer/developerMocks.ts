import { catalogFields, type CatalogReview } from '@/components/developer/catalogEditing'
import type { OwnedAddon } from '@/components/developer/ownedParse'

export type PreviewReviewState = 'none' | CatalogReview['status']

// TODO(backend): Replace this preview-only data with versioned owned-addon reviews and history.
// Keep it visibly labeled and never persist a preview state as a real catalog operation.
export function mockCatalogReview(
  addon: OwnedAddon,
  status: PreviewReviewState
): CatalogReview | null {
  if (status === 'none') return null
  return {
    prNumber: 129,
    htmlUrl: '', // No invented GitHub destinations.
    status,
    proposed: {
      ...catalogFields(addon),
      description: `${addon.description || addon.alias}\nIncludes clearer setup instructions and configuration examples.`,
    },
    feedback:
      status === 'rejected'
        ? 'Please explain the configuration steps more clearly before resubmitting.'
        : null,
  }
}

export const mockReviewHistory = [
  { number: 128, status: 'Approved', date: '2026-08-18' },
  { number: 121, status: 'Withdrawn', date: '2026-08-12' },
]

// TODO(backend): Fetch dated download buckets for the selected addon and period.
export const mockDownloadTrends = {
  '7': [24, 38, 30, 52, 46, 64, 58],
  '30': [48, 67, 54, 78, 92, 83, 106, 99, 128, 116],
}
