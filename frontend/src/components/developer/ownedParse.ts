import type { DeprecationRequest, SubmissionMessage } from '@/components/developer/types.ts'
import type { Deprecation } from '@/lib/deprecation'

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
  library: boolean
  addedAt: string | null
  reviewHistory: ReviewHistoryEntry[]
  webhookEnabled: boolean
  webhookLastEventAt: string | null
  deprecated: boolean
  deprecationReason: string | null
  replacement: string | null
  deprecationRequest: DeprecationRequest | null
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
  library: boolean
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

/** The live deprecation of an owned addon, in the shape the shared deprecation UI takes. */
export function ownedDeprecation(
  addon: Pick<OwnedAddon, 'deprecated' | 'deprecationReason' | 'replacement'>
): Deprecation | null {
  return addon.deprecated
    ? { reason: addon.deprecationReason, replacement: addon.replacement }
    : null
}

export function isFiniteDateString(value: string): boolean {
  return Number.isFinite(new Date(value).getTime())
}
