import { useQuery } from '@tanstack/react-query'

import { getSubmissionDetail } from '@/components/developer/submission.ts'
import { useUserStore } from '@/stores/userStore'

/**
 * Loads the review metadata for one submission (`GET /dev/addon/submissions/{id}`).
 * Editor values stay owned by useDevAddonValues; this hook only reports the
 * submission's status, kind, timestamps, and comment history. `refetch`
 * re-reads the same submission, which is how a view reflects a save it just made.
 */
export function useDevAddonSubmission(id: number) {
  const discordId = useUserStore(state => state.user.discord_id)
  const query = useQuery({
    queryKey: ['dev-addon-submission', discordId, id],
    queryFn: async () => {
      const result = await getSubmissionDetail(id)
      return result.status === 'ok'
        ? { submission: result.submission, error: null }
        : { submission: null, error: result.message }
    },
  })

  return {
    submission: query.data?.submission ?? null,
    error: query.data?.error ?? null,
    loading: query.isPending,
    refetch: query.refetch,
  }
}
