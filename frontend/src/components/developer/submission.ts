import {
  parseSubmissionDetail,
  type ParseSubmissionDetailResult,
} from '@/components/developer/parse.ts'
import { apiClient } from '@/lib/api'

export type { ParseSubmissionDetailResult }

export async function getSubmissionDetail(id: number): Promise<ParseSubmissionDetailResult> {
  try {
    const response = await apiClient.get(`/dev/addon/submissions/${id}`)
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to view this submission.' }
      }
      return { status: 'error', message: 'Unexpected submission response.' }
    }
    return parseSubmissionDetail(response.status, body)
  } catch {
    return { status: 'error', message: 'Unexpected submission response.' }
  }
}
