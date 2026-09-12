import { useEffect, useState } from 'react'

import { getSubmissionDetail } from '@/components/developer/submission.ts'
import type { SubmissionDetail } from '@/components/developer/types.ts'

/**
 * Loads the review metadata for one submission (`GET /dev/addon/submissions/{id}`).
 * Editor values stay owned by useDevAddonValues; this hook only reports the
 * submission's status, kind, timestamps, and comment history. `reloadKey`
 * re-reads the same submission, which is how a view reflects a save it just made.
 */
export function useDevAddonSubmission(id: number, reloadKey = 0) {
  const [currentId, setCurrentId] = useState(id)
  const [currentReloadKey, setCurrentReloadKey] = useState(reloadKey)
  const [result, setResult] = useState<{
    submission: SubmissionDetail | null
    error: string | null
  } | null>(null)

  if (currentId !== id || currentReloadKey !== reloadKey) {
    setCurrentId(id)
    setCurrentReloadKey(reloadKey)
    setResult(null)
  }

  useEffect(() => {
    let cancelled = false
    void (async () => {
      const result = await getSubmissionDetail(id)
      if (cancelled) return
      setResult(
        result.status === 'ok'
          ? { submission: result.submission, error: null }
          : { submission: null, error: result.message }
      )
    })()
    return () => {
      cancelled = true
    }
  }, [id, reloadKey])

  return {
    submission: result?.submission ?? null,
    error: result?.error ?? null,
    loading: result === null,
  }
}
