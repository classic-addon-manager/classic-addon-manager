import { parseAddonSources, type ParseSourcesResult } from '@/components/developer/parse.ts'
import { apiClient } from '@/lib/api'

export async function getAddonSources(): Promise<ParseSourcesResult> {
  try {
    const response = await apiClient.get('/dev/addon/sources')
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to view your addons.' }
      }
      return { status: 'error', message: 'Unexpected sources response.' }
    }
    return parseAddonSources(response.status, body)
  } catch {
    return { status: 'error', message: 'Unexpected sources response.' }
  }
}

type ActionableSubmission =
  | { status: 'found'; id: number }
  | { status: 'none' }
  | { status: 'error'; message: string }

/** Find an open or rejected submission to resume instead of creating a duplicate. */
export async function findActionableSubmission(name: string): Promise<ActionableSubmission> {
  const result = await getAddonSources()
  if (result.status !== 'ok') return { status: 'error', message: result.message }
  const wanted = name.trim().toLowerCase()
  const row = result.sources.submissions.find(
    submission =>
      submission.name.trim().toLowerCase() === wanted &&
      (submission.status === 'open' || submission.status === 'rejected')
  )
  return row ? { status: 'found', id: row.id } : { status: 'none' }
}
