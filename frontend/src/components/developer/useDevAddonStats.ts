import { useEffect, useState } from 'react'

import { getAddonStats } from '@/components/developer/stats.ts'
import type { AddonDeveloperStats } from '@/components/developer/types.ts'

/**
 * Loads owner-scoped statistics for one addon (`GET /dev/addon/{uuid}/stats`).
 * `reloadKey` re-reads the same addon after a failed load.
 */
export function useDevAddonStats(uuid: string, reloadKey = 0) {
  const [currentUuid, setCurrentUuid] = useState(uuid)
  const [currentReloadKey, setCurrentReloadKey] = useState(reloadKey)
  const [result, setResult] = useState<{
    stats: AddonDeveloperStats | null
    error: string | null
  } | null>(null)

  if (currentUuid !== uuid || currentReloadKey !== reloadKey) {
    setCurrentUuid(uuid)
    setCurrentReloadKey(reloadKey)
    setResult(null)
  }

  useEffect(() => {
    let cancelled = false
    void (async () => {
      const loaded = await getAddonStats(uuid)
      if (cancelled) return
      setResult(
        loaded.status === 'ok'
          ? { stats: loaded.stats, error: null }
          : { stats: null, error: loaded.message }
      )
    })()
    return () => {
      cancelled = true
    }
  }, [uuid, reloadKey])

  return {
    stats: result?.stats ?? null,
    error: result?.error ?? null,
    loading: result === null,
  }
}
