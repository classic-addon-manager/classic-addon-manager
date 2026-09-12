import { AlertTriangleIcon } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'

import { getOwnedAddons, type GetOwnedAddonsResult } from '@/components/developer/ownedAddons'
import type { OwnedAddon, OwnedSubmission } from '@/components/developer/ownedParse'
import { toast } from '@/components/ui/toast'

export type OwnedAddonsData = {
  addons: OwnedAddon[]
  submissions: OwnedSubmission[]
}

export function useOwnedAddons(
  enabled: boolean,
  sessionKey: string
): {
  data: OwnedAddonsData | null
  error: string | null
  retry: () => Promise<void>
  removeSubmission: (id: number) => void
} {
  const [currentSessionKey, setCurrentSessionKey] = useState(sessionKey)
  const [data, setData] = useState<OwnedAddonsData | null>(null)
  const [error, setError] = useState<string | null>(null)
  const dataRef = useRef<OwnedAddonsData | null>(null)
  const generationRef = useRef(0)
  const inFlightRef = useRef(false)

  if (currentSessionKey !== sessionKey) {
    setCurrentSessionKey(sessionKey)
    setData(null)
    setError(null)
  }

  useEffect(() => {
    dataRef.current = data
  }, [data])

  const load = useCallback(async (kind: 'user' | 'poll') => {
    if (kind === 'poll' && inFlightRef.current) return
    const my = ++generationRef.current
    inFlightRef.current = true
    try {
      const result = await getOwnedAddons()
      if (my !== generationRef.current) return
      inFlightRef.current = false
      applyResult(kind, result, dataRef.current, setData, setError)
    } finally {
      if (my === generationRef.current) {
        inFlightRef.current = false
      }
    }
  }, [])

  useEffect(() => {
    if (!enabled) {
      generationRef.current += 1
      return
    }

    void load('user')
    const id = window.setInterval(() => {
      void load('poll')
    }, 60_000)

    return () => {
      window.clearInterval(id)
      generationRef.current += 1
    }
  }, [enabled, sessionKey, load])

  const retry = () => {
    setError(null)
    return load('user')
  }

  const removeSubmission = (id: number) => {
    setData(current =>
      current
        ? {
            addons: current.addons,
            submissions: current.submissions.filter(submission => submission.id !== id),
          }
        : current
    )
  }

  return { data, error, retry, removeSubmission }
}

function applyResult(
  kind: 'user' | 'poll',
  result: GetOwnedAddonsResult,
  current: OwnedAddonsData | null,
  setData: (data: OwnedAddonsData | null) => void,
  setError: (error: string | null) => void
) {
  if (result.status === 'ok') {
    setData({ addons: result.addons, submissions: result.submissions })
    setError(null)
    return
  }

  if (kind === 'poll') return

  if (current === null) {
    setError(result.message)
    return
  }

  toast({ title: 'Error', description: result.message, icon: AlertTriangleIcon })
}
