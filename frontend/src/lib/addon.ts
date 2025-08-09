import { AlertTriangleIcon, ThumbsDownIcon, ThumbsUpIcon } from 'lucide-react'

import { toast } from '@/components/ui/toast.tsx'
import { apiClient } from '@/lib/api.ts'

export async function rateAddon(
  addonName: string,
  addonAlias: string,
  newRating: number,
  currentRating: number,
  setRating: (rating: number) => void
): Promise<void> {
  if (currentRating === newRating) return

  const res = await apiClient.post(`/addon/${addonName}/rate`, {
    is_like: newRating === 1,
  })

  if (res.status !== 200) {
    toast({
      title: 'Error',
      description: 'Failed to rate addon, try again later',
      icon: AlertTriangleIcon,
    })
    return
  }

  if (newRating === 1) {
    toast({
      title: 'Addon rated',
      description: `You liked ${addonAlias}`,
      icon: ThumbsUpIcon,
    })
  } else {
    toast({
      title: 'Addon rated',
      description: `You disliked ${addonAlias}`,
      icon: ThumbsDownIcon,
    })
  }

  setRating(newRating)
}
