import { parseAddonStats, type ParseAddonStatsResult } from '@/components/developer/parse.ts'

import { fetchParsed } from './fetchJson.ts'

export type { ParseAddonStatsResult }

export async function getAddonStats(uuid: string): Promise<ParseAddonStatsResult> {
  return fetchParsed(
    `/dev/addon/${encodeURIComponent(uuid)}/stats`,
    {},
    parseAddonStats,
    'Sign in to view addon statistics.',
    'Unexpected statistics response.'
  )
}
