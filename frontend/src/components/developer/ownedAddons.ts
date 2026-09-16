import {
  isFiniteDateString,
  type OwnedAddon,
  type OwnedSubmission,
  type OwnedSubmissionPayload,
  type ReviewHistoryEntry,
  type ReviewHistoryTone,
} from '@/components/developer/ownedParse'
import { getAddonSources } from '@/components/developer/sources.ts'
import type { AddonSources, SourceAddon, SourceSubmission } from '@/components/developer/types.ts'
import { type AddonManifest, RemoteAddonService } from '@/lib/wails'

export type GetOwnedAddonsResult =
  | { status: 'ok'; addons: OwnedAddon[]; submissions: OwnedSubmission[] }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

export async function getOwnedAddons(): Promise<GetOwnedAddonsResult> {
  const [result, manifests] = await Promise.all([
    getAddonSources(),
    // Catalog enrichment is best-effort: ownership still loads when it is unavailable.
    RemoteAddonService.GetAddonManifest().catch(() => []),
  ])
  if (result.status !== 'ok') return result
  return { status: 'ok', ...sourcesToOwned(result.sources, manifests) }
}

export function sourcesToOwned(
  sources: AddonSources,
  manifests: Pick<AddonManifest, 'name' | 'repo' | 'branch'>[] = []
): {
  addons: OwnedAddon[]
  submissions: OwnedSubmission[]
} {
  const manifestByName = new Map(manifests.map(manifest => [manifest.name, manifest]))
  const published = new Set(sources.addons.map(addon => addon.name))
  const historyByName = new Map<string, SourceSubmission[]>()
  const standalone: SourceSubmission[] = []

  for (const submission of sources.submissions) {
    if (submission.kind === 'update' && published.has(submission.name)) {
      const list = historyByName.get(submission.name)
      if (list) list.push(submission)
      else historyByName.set(submission.name, [submission])
    } else {
      standalone.push(submission)
    }
  }

  return {
    addons: sources.addons.map(addon =>
      toOwnedAddon(addon, manifestByName.get(addon.name), historyByName.get(addon.name) ?? [])
    ),
    submissions: [
      ...standalone.map(toOwnedSubmission),
      ...[...historyByName.values()].flat().map(toOwnedSubmission),
    ],
  }
}

function toOwnedAddon(
  addon: SourceAddon,
  manifest: Pick<AddonManifest, 'name' | 'repo' | 'branch'> | undefined,
  submissions: SourceSubmission[]
): OwnedAddon {
  return {
    uuid: addon.uuid,
    name: addon.name,
    alias: addon.alias,
    repo: manifest?.repo ?? '',
    branch: manifest?.branch || null,
    author: '',
    description: '',
    tags: [],
    downloads: addon.downloads,
    likePercentage: null,
    warning: null,
    addedAt: null,
    reviewHistory: toReviewHistory(submissions),
  }
}

function toReviewHistory(submissions: SourceSubmission[]): ReviewHistoryEntry[] {
  return submissions
    .slice()
    .sort((a, b) => b.id - a.id)
    .map(submission => ({
      number: submission.id,
      status: reviewStatus(submission.status),
      tone: statusTone(submission.status),
      date: reviewDate(submission.createdAt),
      submissionId: submission.id,
    }))
}

const REVIEW_STATUS_LABELS: Record<string, string> = {
  open: 'In review',
  in_review: 'In review',
  rejected: 'Rejected',
  approved: 'Approved',
}

const REVIEW_STATUS_TONES: Record<string, ReviewHistoryTone> = {
  open: 'review',
  in_review: 'review',
  rejected: 'rejected',
  approved: 'approved',
}

/**
 * Tone for a submission history row. A status the backend sends that we do not
 * map stays visibly distinct instead of passing through as an unremarkable string.
 */
export function statusTone(status: string | undefined): ReviewHistoryTone {
  // A missing status cannot be asserted as a real outcome, so it stays unknown.
  if (!status) return 'unknown'
  return REVIEW_STATUS_TONES[status] ?? 'unknown'
}

function reviewStatus(status: string | undefined): string {
  if (!status) return 'Unknown'
  return REVIEW_STATUS_LABELS[status] ?? status
}

function reviewDate(createdAt: string | undefined): string | null {
  return createdAt && isFiniteDateString(createdAt) ? createdAt : null
}

function emptyPayload(name: string): OwnedSubmissionPayload {
  return {
    name,
    alias: name,
    description: '',
    author: '',
    repo: '',
    branch: '',
    tags: [],
    keywords: [],
    dependencies: [],
    kofi: '',
  }
}

function toOwnedSubmission(submission: SourceSubmission): OwnedSubmission {
  return {
    id: submission.id,
    kind: submission.kind,
    status: submission.status === 'rejected' ? 'rejected' : 'open',
    title: submission.name,
    createdAt:
      submission.createdAt && isFiniteDateString(submission.createdAt)
        ? submission.createdAt
        : null,
    payload: emptyPayload(submission.name),
    messages: [],
  }
}
