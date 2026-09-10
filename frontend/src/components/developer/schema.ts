import { parseAddonSchema, type ParseSchemaResult } from '@/components/developer/parse.ts'
import type { AddonSchema } from '@/components/developer/types.ts'
import { apiClient } from '@/lib/api'

export type { ParseSchemaResult }

let cached: AddonSchema | null = null

export async function getAddonSchema(): Promise<ParseSchemaResult> {
  try {
    const response = await apiClient.get('/dev/addon/schema')
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to edit addon declarations.' }
      }
      return { status: 'error', message: 'Unexpected schema response.' }
    }
    const parsed = parseAddonSchema(response.status, body)
    if (parsed.status === 'ok') cached = parsed.schema
    return parsed
  } catch {
    return { status: 'error', message: 'Unexpected schema response.' }
  }
}

export async function requireAddonSchema(): Promise<AddonSchema | null> {
  if (cached) return cached
  const result = await getAddonSchema()
  return result.status === 'ok' ? result.schema : null
}
