import { useQuery } from '@tanstack/react-query'

import { getAddonStats } from '@/components/developer/stats.ts'
import { useUserStore } from '@/stores/userStore'

/**
 * Loads owner-scoped statistics for one addon (`GET /dev/addon/{uuid}/stats`).
 * `refetch` re-reads the same addon after a failed load.
 */
export function useDevAddonStats(uuid: string) {
  const discordId = useUserStore(state => state.user.discord_id)
  const query = useQuery({
    queryKey: ['dev-addon-stats', discordId, uuid],
    // Snapshots are taken hourly, 15 minutes avoids refetching on every mount.
    staleTime: 15 * 60 * 1000,
    queryFn: async () => {
      const loaded = await getAddonStats(uuid)
      return loaded.status === 'ok'
        ? { stats: loaded.stats, error: null }
        : { stats: null, error: loaded.message }
    },
  })

  return {
    stats: query.data?.stats ?? null,
    error: query.data?.error ?? null,
    loading: query.isPending,
    refetch: query.refetch,
  }
}
