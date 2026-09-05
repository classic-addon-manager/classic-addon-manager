import { InfoIcon } from 'lucide-react'

import type { OwnedAddon } from '@/components/developer/ownedParse'
import { toast } from '@/components/ui/toast'

export type CatalogFields = Pick<OwnedAddon, 'alias' | 'description' | 'repo' | 'tags'> & {
  branch: string
}

// TODO(backend): Add an owned-addon review endpoint, linking reviews by immutable addon name.
// Published catalog values must remain separate from the proposed, versioned review payload.
export interface CatalogReview {
  prNumber: number
  htmlUrl: string
  status: 'in_review' | 'approved' | 'rejected' | 'withdrawn'
  proposed: CatalogFields
  feedback: string | null
}

export function catalogFields(addon: OwnedAddon): CatalogFields {
  return {
    alias: addon.alias,
    description: addon.description,
    repo: addon.repo,
    branch: addon.branch ?? '',
    tags: [...addon.tags],
  }
}

export const catalogFieldLabels: Record<keyof CatalogFields, string> = {
  alias: 'Display name',
  description: 'Description',
  repo: 'Repository',
  branch: 'Branch',
  tags: 'Tags',
}

export function changedCatalogFields(before: CatalogFields, after: CatalogFields) {
  return (Object.keys(catalogFieldLabels) as (keyof CatalogFields)[]).filter(key => {
    if (key === 'tags')
      return [...before.tags].sort().join('\0') !== [...after.tags].sort().join('\0')
    return before[key] !== after[key]
  })
}

export function backendUnavailable(action: string) {
  toast({
    title: action,
    description: 'Not implemented in the backend yet.',
    icon: InfoIcon,
  })
}
