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
  const [currentSource, setCurrentSource] = useState(source)
  const [result, setResult] = useState<{
    values: DeclarationValues | null
    kind: DeclarationKind | null
    error: string | null
  } | null>(null)

  if (sourceCacheKey(currentSource) !== sourceCacheKey(source)) {
    setCurrentSource(source)
    setResult(null)
  }

  useEffect(() => {
    let cancelled = false
    void (async () => {
      const schema = await requireAddonSchema()
      if (cancelled) return
      if (!schema) {
        setResult({
          values: null,
          kind: null,
          error: 'This source is unavailable.',
        })
        return
      }
      const loaded = await getAddonValues(currentSource, schema)
      if (cancelled) return
      if (loaded.status === 'ok') {
        setResult({
          kind: loaded.kind,
          values: loaded.values,
          error: null,
        })
        return
      }
      setResult({
        values: null,
        kind: null,
        error:
          loaded.status === 'unauthorized' ||
          loaded.status === 'error' ||
          loaded.status === 'not_found'
            ? loaded.message
            : 'This source is unavailable.',
      })
    })()
    return () => {
      cancelled = true
    }
  }, [currentSource])

  if (result === null) {
    return { values: null, kind: null, error: null, loading: true }
  }

  return { values: result.values, kind: result.kind, error: result.error, loading: false }
}
