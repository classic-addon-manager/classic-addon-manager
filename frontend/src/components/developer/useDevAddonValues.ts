import { useEffect, useState } from 'react'

import { requireAddonSchema } from '@/components/developer/schema.ts'
import type {
  DeclarationKind,
  DeclarationValues,
  EditorSource,
} from '@/components/developer/types.ts'
import { getAddonValues } from '@/components/developer/values.ts'

/** Cache key for a source; an absent source (null) is its own key and never fetches. */
function sourceCacheKey(source: EditorSource | null): string {
  if (source === null) return 'none'
  if (source.type === 'new') return 'new'
  if (source.type === 'addon') return `addon:${source.uuid}`
  return `submission:${source.id}`
}

/**
 * Loads the editor values for one source. `reloadKey` re-reads the same source,
 * which is how a view reflects a save it just made. A null source (a submission
 * the view may not have) loads nothing and reports no values.
 */
export function useDevAddonValues(source: EditorSource | null, reloadKey = 0) {
  const [currentSource, setCurrentSource] = useState(source)
  const [currentReloadKey, setCurrentReloadKey] = useState(reloadKey)
  const [result, setResult] = useState<{
    values: DeclarationValues | null
    kind: DeclarationKind | null
    error: string | null
  } | null>(null)

  if (sourceCacheKey(currentSource) !== sourceCacheKey(source) || currentReloadKey !== reloadKey) {
    setCurrentSource(source)
    setCurrentReloadKey(reloadKey)
    setResult(null)
  }

  useEffect(() => {
    if (currentSource === null) return
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
  }, [currentSource, currentReloadKey])

  if (currentSource === null) {
    return { values: null, kind: null, error: null, loading: false }
  }

  if (result === null) {
    return { values: null, kind: null, error: null, loading: true }
  }

  return { values: result.values, kind: result.kind, error: result.error, loading: false }
}
