import { parseAddonValues, type ParseValuesResult } from '@/components/developer/parse.ts'
import type { AddonSchema, EditorSource } from '@/components/developer/types.ts'

import { fetchParsed } from './fetchJson.ts'

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
  return fetchParsed(
    `/dev/addon/values?${query}`,
    {},
    (status, body) => parseAddonValues(status, body, schema),
    'Sign in to load addon values.',
    'Unexpected values response.'
  )
}
