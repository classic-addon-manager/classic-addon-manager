import {
  parseSubmissionDetail,
  type ParseSubmissionDetailResult,
} from '@/components/developer/parse.ts'

import { fetchParsed } from './fetchJson.ts'

export type { ParseSubmissionDetailResult }

export async function getSubmissionDetail(id: number): Promise<ParseSubmissionDetailResult> {
  return fetchParsed(
    `/dev/addon/submissions/${id}`,
    {},
    parseSubmissionDetail,
    'Sign in to view this submission.',
    'Unexpected submission response.'
  )
}
