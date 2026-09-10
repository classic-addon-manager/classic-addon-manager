import { parseAddonValues, type ParseValuesResult } from '@/components/developer/parse.ts'
import type { AddonSchema, EditorSource } from '@/components/developer/types.ts'
import { apiClient } from '@/lib/api'

export type { ParseValuesResult }

export async function getAddonValues(
  source: EditorSource,
  schema: AddonSchema
): Promise<ParseValuesResult> {
  const query =
    source.type === 'new'
      ? 'source=new'
      : source.type === 'addon'
        ? `addon_uuid=${encodeURIComponent(source.uuid)}`
        : `submission_id=${source.id}`
  try {
    const response = await apiClient.get(`/dev/addon/values?${query}`)
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to load addon values.' }
      }
      return { status: 'error', message: 'Unexpected values response.' }
    }
    return parseAddonValues(response.status, body, schema)
  } catch {
    return { status: 'error', message: 'Unexpected values response.' }
  }
}
