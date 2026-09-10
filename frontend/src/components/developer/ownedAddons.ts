import type {
  OwnedAddon,
  OwnedSubmission,
  OwnedSubmissionPayload,
} from '@/components/developer/ownedParse'
import { getAddonSources } from '@/components/developer/sources.ts'
import type { SourceAddon, SourceSubmission } from '@/components/developer/types.ts'

export type GetOwnedAddonsResult =
  | { status: 'ok'; addons: OwnedAddon[]; submissions: OwnedSubmission[] }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

export async function getOwnedAddons(): Promise<GetOwnedAddonsResult> {
  const result = await getAddonSources()
  if (result.status !== 'ok') return result
  return {
    status: 'ok',
    addons: result.sources.addons.map(toOwnedAddon),
    submissions: result.sources.submissions.map(toOwnedSubmission),
  }
}

function toOwnedAddon(addon: SourceAddon): OwnedAddon {
  return {
    uuid: addon.uuid,
    name: addon.name,
    alias: addon.alias,
    repo: '',
    branch: null,
    author: '',
    description: '',
    tags: [],
    downloads: 0,
    likePercentage: null,
    warning: null,
    addedAt: null,
  }
}

function emptyPayload(name: string): OwnedSubmissionPayload {
  return {
    name,
    alias: name,
    description: '',
    author: '',
    repo: '',
    branch: '',
    tags: [],
    keywords: [],
    dependencies: [],
    kofi: '',
  }
}

function toOwnedSubmission(submission: SourceSubmission): OwnedSubmission {
  return {
    id: submission.id,
    kind: submission.kind,
    status: 'open',
    title: submission.name,
    createdAt: null,
    payload: emptyPayload(submission.name),
    messages: [],
  }
}
