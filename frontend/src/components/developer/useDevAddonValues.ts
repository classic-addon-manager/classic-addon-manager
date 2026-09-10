import { useEffect, useState } from 'react'

import { requireAddonSchema } from '@/components/developer/schema.ts'
import type {
  DeclarationKind,
  DeclarationValues,
  EditorSource,
} from '@/components/developer/types.ts'
import { getAddonValues } from '@/components/developer/values.ts'

export function sourceCacheKey(source: EditorSource): string {
  if (source.type === 'new') return 'new'
  if (source.type === 'addon') return `addon:${source.uuid}`
  return `submission:${source.id}`
}

export function useDevAddonValues(source: EditorSource) {
  const [values, setValues] = useState<DeclarationValues | null>(null)
  const [kind, setKind] = useState<DeclarationKind | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const key = sourceCacheKey(source)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    setValues(null)
    setError(null)
    void (async () => {
      const schema = await requireAddonSchema()
      if (cancelled) return
      if (!schema) {
        setError('This source is unavailable.')
        setLoading(false)
        return
      }
      const result = await getAddonValues(source, schema)
      if (cancelled) return
      setLoading(false)
      if (result.status === 'ok') {
        setKind(result.kind)
        setValues(result.values)
        return
      }
      setError(
        result.status === 'unauthorized' ||
          result.status === 'error' ||
          result.status === 'not_found'
          ? result.message
          : 'This source is unavailable.'
      )
    })()
    return () => {
      cancelled = true
    }
  }, [key])

  return { values, kind, error, loading }
}
