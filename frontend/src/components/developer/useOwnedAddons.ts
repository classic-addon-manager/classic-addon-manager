import { useQuery, useQueryClient } from '@tanstack/react-query'
import { AlertTriangleIcon } from 'lucide-react'

import { getOwnedAddons } from '@/components/developer/ownedAddons'
import type { OwnedAddon, OwnedSubmission } from '@/components/developer/ownedParse'
import { toast } from '@/components/ui/toast'
import { useUserStore } from '@/stores/userStore'

export type OwnedAddonsData = {
  addons: OwnedAddon[]
  submissions: OwnedSubmission[]
}

export function useOwnedAddons(enabled: boolean): {
  data: OwnedAddonsData | null
  error: string | null
  retry: () => Promise<void>
  removeSubmission: (id: number) => void
  lastAttemptFailed: boolean
} {
  const queryClient = useQueryClient()
  const discordId = useUserStore(state => state.user.discord_id)
  const queryKey = ['owned-addons', discordId] as const
  const query = useQuery({
    queryKey,
    enabled,
    refetchInterval: 60_000,
    retry: false,
    queryFn: async (): Promise<OwnedAddonsData> => {
      const result = await getOwnedAddons()
      if (result.status !== 'ok') throw new Error(result.message)
      return { addons: result.addons, submissions: result.submissions }
    },
  })

  const retry = async () => {
    // Refresh covers the open detail panes too, not just the list.
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['dev-addon-stats'] }),
      queryClient.invalidateQueries({ queryKey: ['dev-addon-submission'] }),
      queryClient.invalidateQueries({ queryKey: ['dev-addon-values'] }),
    ])
    const result = await query.refetch()
    // Poll failures stay silent, a user-triggered retry surfaces the error.
    if (result.isError && result.data) {
      toast({ title: 'Error', description: result.error.message, icon: AlertTriangleIcon })
    }
  }

  const removeSubmission = (id: number) => {
    queryClient.setQueryData<OwnedAddonsData>(queryKey, current =>
      current
        ? {
            addons: current.addons,
            submissions: current.submissions.filter(submission => submission.id !== id),
          }
        : current
    )
  }

  return {
    data: query.data ?? null,
    // Poll failures keep the last data and stay silent, the error only shows
    // when there is nothing to display.
    error: query.data === undefined && query.isError ? query.error.message : null,
    retry,
    removeSubmission,
    lastAttemptFailed: query.isError,
  }
}
