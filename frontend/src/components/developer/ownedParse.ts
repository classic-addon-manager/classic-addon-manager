import type { SubmissionMessage } from '@/components/developer/types.ts'

/** Visual family of a submission history status: drives the chip color and icon. */
export type ReviewHistoryTone = 'review' | 'approved' | 'rejected' | 'unknown'

export type ReviewHistoryEntry = {
  number: number
  status: string
  tone: ReviewHistoryTone
  date: string | null
  submissionId: number
}

export type OwnedAddon = {
  uuid: string
  name: string
  alias: string
  repo: string
  branch: string | null
  icon: string | null
  author: string
  description: string
  tags: string[]
  downloads: number
  likePercentage: number | null
  warning: string | null
  addedAt: string | null
  reviewHistory: ReviewHistoryEntry[]
}

export type OwnedSubmissionPayload = {
  name: string
  alias: string
  description: string
  author: string
  repo: string
  branch: string
  tags: string[]
  keywords: string[]
  dependencies: string[]
  kofi: string
}

export type OwnedSubmission = {
  id: number
  kind?: 'new' | 'update'
  status: 'open' | 'rejected'
  title: string
  createdAt: string | null
  payload: OwnedSubmissionPayload
  messages: SubmissionMessage[]
}

export function isFiniteDateString(value: string): boolean {
  return Number.isFinite(new Date(value).getTime())
}
