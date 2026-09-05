import { AlertTriangleIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

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
  retry: () => void
} {
  const [data, setData] = useState<OwnedAddonsData | null>(null)
  const [error, setError] = useState<string | null>(null)
  const dataRef = useRef(data)
  dataRef.current = data
  const generationRef = useRef(0)
  const inFlightRef = useRef(false)

  const loadRef = useRef<(kind: 'user' | 'poll') => void>(() => {})
  loadRef.current = (kind: 'user' | 'poll') => {
    void (async () => {
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
    })()
  }

  useEffect(() => {
    setData(null)
    setError(null)
    generationRef.current += 1
  }, [sessionKey])

  useEffect(() => {
    if (!enabled) {
      generationRef.current += 1
      return
    }

    void loadRef.current('user')
    const id = window.setInterval(() => {
      void loadRef.current('poll')
    }, 60_000)

    return () => {
      window.clearInterval(id)
      generationRef.current += 1
    }
  }, [enabled, sessionKey])

  const retry = () => {
    setError(null)
    void loadRef.current('user')
  }

  return { data, error, retry }
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
