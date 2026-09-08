export const APPROVED_TAGS = [
  'Automation',
  'Chat',
  'Combat',
  'Economy',
  'Fishing',
  'Inventory',
  'Library',
  'Map',
  'Other',
  'PvE',
  'PvP',
  'QoL',
  'Raid',
  'Social',
  'UI',
] as const

export type ApprovedTag = (typeof APPROVED_TAGS)[number]

export interface PublishFormState {
  name: string
  alias: string
  description: string
  author: string
  repo: string
  branch: string
  tags: ApprovedTag[]
  keywords: string
  dependencies: string[]
  kofi: string
}

export const INITIAL_PUBLISH_FORM: PublishFormState = {
  name: '',
  alias: '',
  description: '',
  author: '',
  repo: '',
  branch: '',
  tags: [],
  keywords: '',
  dependencies: [],
  kofi: '',
}

export function isPublishFormDirty(
  form: PublishFormState,
  baseline: PublishFormState = INITIAL_PUBLISH_FORM
): boolean {
  return (
    form.name !== baseline.name ||
    form.alias !== baseline.alias ||
    form.description !== baseline.description ||
    form.author !== baseline.author ||
    form.repo !== baseline.repo ||
    form.branch !== baseline.branch ||
    form.keywords !== baseline.keywords ||
    form.kofi !== baseline.kofi ||
    form.tags.length !== baseline.tags.length ||
    form.tags.some((tag, index) => tag !== baseline.tags[index]) ||
    form.dependencies.length !== baseline.dependencies.length ||
    form.dependencies.some((dep, index) => dep !== baseline.dependencies[index])
  )
}

export function publishFormFromPayload(payload: {
  name: string
  alias: string
  description: string
  author: string
  repo: string
  branch: string
  tags: string[]
  keywords: string[]
  dependencies: string[]
  kofi: string
}): PublishFormState {
  return {
    name: payload.name,
    alias: payload.alias,
    description: payload.description,
    author: payload.author,
    repo: payload.repo,
    branch: payload.branch,
    tags: payload.tags
      .filter((tag): tag is ApprovedTag => (APPROVED_TAGS as readonly string[]).includes(tag))
      .slice(0, 3),
    keywords: payload.keywords.join(' '),
    dependencies: [...payload.dependencies],
    kofi: payload.kofi,
  }
}
