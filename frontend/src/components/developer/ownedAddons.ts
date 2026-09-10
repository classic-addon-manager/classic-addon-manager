import { mockReviewHistory } from '@/components/developer/developerMocks'
import {
  isFiniteDateString,
  type OwnedAddon,
  type OwnedSubmission,
  type OwnedSubmissionPayload,
  type ReviewHistoryEntry,
} from '@/components/developer/ownedParse'
import { getAddonSources } from '@/components/developer/sources.ts'
import type { AddonSources, SourceAddon, SourceSubmission } from '@/components/developer/types.ts'

export type GetOwnedAddonsResult =
  | { status: 'ok'; addons: OwnedAddon[]; submissions: OwnedSubmission[] }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

/** Placeholder used when a source submission has no date. Visibly labeled as mocked. */
const MOCK_REVIEW_DATE = '2026-08-18'

export async function getOwnedAddons(): Promise<GetOwnedAddonsResult> {
  const result = await getAddonSources()
  if (result.status !== 'ok') return result
  return { status: 'ok', ...sourcesToOwned(result.sources) }
}

export function sourcesToOwned(sources: AddonSources): {
  addons: OwnedAddon[]
  submissions: OwnedSubmission[]
} {
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
    addons: sources.addons.map(addon => toOwnedAddon(addon, historyByName.get(addon.name) ?? [])),
    submissions: [
      ...standalone.map(toOwnedSubmission),
      ...[...historyByName.values()].flat().map(toOwnedSubmission),
    ],
  }
}

function toOwnedAddon(addon: SourceAddon, submissions: SourceSubmission[]): OwnedAddon {
  return {
    uuid: addon.uuid,
    name: addon.name,
    alias: addon.alias,
    repo: '',
    branch: null,
    author: '',
    description: '',
    tags: [],
    downloads: 0,
    likePercentage: null,
    warning: null,
    addedAt: null,
    reviewHistory: toReviewHistory(submissions),
  }
}

function toReviewHistory(submissions: SourceSubmission[]): ReviewHistoryEntry[] {
  if (submissions.length === 0) {
    // TODO(backend): Drop fallback rows once closed reviews are returned with the addon.
    return mockReviewHistory.map(entry => ({
      number: entry.number,
      status: entry.status,
      date: entry.date,
      submissionId: null,
      statusMocked: true,
      dateMocked: true,
    }))
  }
  return submissions
    .slice()
    .sort((a, b) => b.id - a.id)
    .map(submission => {
      const status = reviewStatus(submission.status)
      const date = reviewDate(submission.createdAt)
      return {
        number: submission.id,
        status: status.value,
        date: date.value,
        submissionId: submission.id,
        statusMocked: status.mocked,
        dateMocked: date.mocked,
      }
    })
}

const REVIEW_STATUS_LABELS: Record<string, string> = {
  open: 'In review',
  in_review: 'In review',
  rejected: 'Rejected',
  approved: 'Approved',
  withdrawn: 'Withdrawn',
}

function reviewStatus(status: string | undefined): { value: string; mocked: boolean } {
  if (!status) return { value: 'In review', mocked: true }
  return { value: REVIEW_STATUS_LABELS[status] ?? status, mocked: false }
}

function reviewDate(createdAt: string | undefined): { value: string; mocked: boolean } {
  if (createdAt && isFiniteDateString(createdAt)) return { value: createdAt, mocked: false }
  return { value: MOCK_REVIEW_DATE, mocked: true }
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
