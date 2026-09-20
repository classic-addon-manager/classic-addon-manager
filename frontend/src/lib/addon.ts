import { useIsMutating, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { AlertTriangleIcon, ThumbsDownIcon, ThumbsUpIcon } from 'lucide-react'

import { toast } from '@/components/ui/toast.tsx'
import { apiClient } from '@/lib/api.ts'
import { useUserStore } from '@/stores/userStore'

type Rating = -1 | 0 | 1

export function useAddonRating(addonName: string, addonAlias: string, enabled = true) {
  const user = useUserStore(state => state.user)
  const client = useQueryClient()
  const queryKey = ['addon-rating', user.discord_id, addonName] as const
  const mutationKey = queryKey
  const isSaving = useIsMutating({ mutationKey, exact: true }) > 0
  const query = useQuery({
    queryKey,
    enabled: enabled && !!user.discord_id,
    staleTime: 5 * 60 * 1000,
    queryFn: async (): Promise<Rating> => {
      const response = await apiClient.get(`/addon/${addonName}/my-rating`)
      if (response.status !== 200) throw new Error('Failed to fetch rating')
      const result = await response.json()
      const rating = result.data?.rating
      if (rating !== -1 && rating !== 0 && rating !== 1) {
        throw new Error('Invalid addon rating')
      }
      return rating
    },
  })
  const mutation = useMutation({
    mutationKey,
    onMutate: async () => {
      await client.cancelQueries({ queryKey, exact: true })
    },
    mutationFn: async (newRating: -1 | 1) => {
      if (useUserStore.getState().user !== user) throw new Error('Account changed')
      const response = await apiClient.post(`/addon/${addonName}/rate`, {
        is_like: newRating === 1,
      })
      if (response.status !== 200) throw new Error('Failed to rate addon')
      return newRating
    },
    onSuccess: async newRating => {
      // A completed write must not restore private data after an account change.
      if (useUserStore.getState().user !== user) return
      await client.cancelQueries({ queryKey, exact: true })
      if (useUserStore.getState().user !== user) return
      client.setQueryData(queryKey, newRating)
      toast({
        title: 'Addon rated',
        description: `You ${newRating === 1 ? 'liked' : 'disliked'} ${addonAlias}`,
        icon: newRating === 1 ? ThumbsUpIcon : ThumbsDownIcon,
      })
      await client.invalidateQueries({ queryKey, exact: true })
    },
    onError: () => {
      if (useUserStore.getState().user !== user) return
      toast({
        title: 'Error',
        description: 'Failed to rate addon, try again later',
        icon: AlertTriangleIcon,
      })
    },
  })

  const rateAddon = async (newRating: number): Promise<boolean> => {
    if (
      !enabled ||
      !user.discord_id ||
      (newRating !== -1 && newRating !== 1) ||
      client.getQueryData(queryKey) === newRating ||
      client.isMutating({ mutationKey, exact: true }) > 0
    ) {
      return false
    }
    try {
      await mutation.mutateAsync(newRating)
      return useUserStore.getState().user === user
    } catch {
      return false
    }
  }

  return { rating: query.data ?? 0, isLoading: query.isLoading, isSaving, rateAddon }
}
