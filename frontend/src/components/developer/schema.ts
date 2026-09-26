import { parseAddonSchema, type ParseSchemaResult } from '@/components/developer/parse.ts'
import type { AddonSchema, DeclarationValues } from '@/components/developer/types.ts'

import { fetchParsed } from './fetchJson.ts'

export type { ParseSchemaResult }

let cached: AddonSchema | null = null

export async function getAddonSchema(): Promise<ParseSchemaResult> {
  const parsed = await fetchParsed(
    '/dev/addon/schema',
    {},
    parseAddonSchema,
    'Sign in to edit addon declarations.',
    'Unexpected schema response.'
  )
  if (parsed.status === 'ok') cached = parsed.schema
  return parsed
}

export async function requireAddonSchema(): Promise<AddonSchema | null> {
  if (cached) return cached
  const result = await getAddonSchema()
  return result.status === 'ok' ? result.schema : null
}

export function schemaZeroValues(schema: AddonSchema): DeclarationValues {
  const values: DeclarationValues = {}
  for (const field of schema.fields) {
    values[field.key] =
      field.widget === 'checkbox'
        ? false
        : field.widget === 'text' || field.widget === 'textarea'
          ? ''
          : []
  }
  return values
}
