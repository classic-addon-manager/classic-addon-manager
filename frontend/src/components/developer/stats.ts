import { parseAddonStats, type ParseAddonStatsResult } from '@/components/developer/parse.ts'
import { apiClient } from '@/lib/api'

export type { ParseAddonStatsResult }

export async function getAddonStats(uuid: string): Promise<ParseAddonStatsResult> {
  try {
    const response = await apiClient.get(`/dev/addon/${encodeURIComponent(uuid)}/stats`)
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to view addon statistics.' }
      }
      return { status: 'error', message: 'Unexpected statistics response.' }
    }
    return parseAddonStats(response.status, body)
  } catch {
    return { status: 'error', message: 'Unexpected statistics response.' }
  }
}
