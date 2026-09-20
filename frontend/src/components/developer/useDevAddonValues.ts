import { useQuery } from '@tanstack/react-query'

import { requireAddonSchema } from '@/components/developer/schema.ts'
import type { EditorSource } from '@/components/developer/types.ts'
import { getAddonValues } from '@/components/developer/values.ts'
import { useUserStore } from '@/stores/userStore'

/** Cache key for a source; an absent source (null) is its own key and never fetches. */
function sourceCacheKey(source: EditorSource | null): string {
  if (source === null) return 'none'
  if (source.type === 'new') return 'new'
  if (source.type === 'addon') return `addon:${source.uuid}`
  return `submission:${source.id}`
}

/**
 * Loads the editor values for one source. `refetch` re-reads the same source,
 * which is how a view reflects a save it just made. A null source (a submission
 * the view may not have) loads nothing and reports no values.
 */
export function useDevAddonValues(source: EditorSource | null) {
  const discordId = useUserStore(state => state.user.discord_id)
  const query = useQuery({
    queryKey: ['dev-addon-values', discordId, sourceCacheKey(source)],
    enabled: source !== null,
    queryFn: async () => {
      if (source === null) {
        return { values: null, kind: null, error: null }
      }
      const schema = await requireAddonSchema()
      if (!schema) {
        return { values: null, kind: null, error: 'This source is unavailable.' }
      }
      const loaded = await getAddonValues(source, schema)
      if (loaded.status === 'ok') {
        return { kind: loaded.kind, values: loaded.values, error: null }
      }
      return {
        values: null,
        kind: null,
        error:
          loaded.status === 'unauthorized' ||
          loaded.status === 'error' ||
          loaded.status === 'not_found'
            ? loaded.message
            : 'This source is unavailable.',
      }
    },
  })

  return {
    values: query.data?.values ?? null,
    kind: query.data?.kind ?? null,
    error: query.data?.error ?? null,
    loading: source !== null && query.isPending,
    refetch: query.refetch,
  }
}
