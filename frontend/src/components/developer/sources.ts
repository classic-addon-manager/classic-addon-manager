import { parseAddonSources, type ParseSourcesResult } from '@/components/developer/parse.ts'
import { apiClient } from '@/lib/api'

export type { ParseSourcesResult }

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
