import { useEffect, useState } from 'react'

import { getSubmissionDetail } from '@/components/developer/submission.ts'
import type { SubmissionDetail } from '@/components/developer/types.ts'

/**
 * Loads the review metadata for one submission (`GET /dev/addon/submissions/{id}`).
 * Editor values stay owned by useDevAddonValues; this hook only reports the
 * submission's status, kind, timestamps, and comment history.
 */
export function useDevAddonSubmission(id: number) {
  const [currentId, setCurrentId] = useState(id)
  const [submission, setSubmission] = useState<SubmissionDetail | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  if (currentId !== id) {
    setCurrentId(id)
    setLoading(true)
    setSubmission(null)
    setError(null)
  }

  useEffect(() => {
    let cancelled = false
    void (async () => {
      const result = await getSubmissionDetail(id)
      if (cancelled) return
      setLoading(false)
      if (result.status === 'ok') {
        setSubmission(result.submission)
        return
      }
      setError(result.message)
    })()
    return () => {
      cancelled = true
    }
  }, [id])

  return { submission, error, loading }
}
