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

export function isPublishFormDirty(form: PublishFormState): boolean {
  return (
    form.name !== INITIAL_PUBLISH_FORM.name ||
    form.alias !== INITIAL_PUBLISH_FORM.alias ||
    form.description !== INITIAL_PUBLISH_FORM.description ||
    form.author !== INITIAL_PUBLISH_FORM.author ||
    form.repo !== INITIAL_PUBLISH_FORM.repo ||
    form.branch !== INITIAL_PUBLISH_FORM.branch ||
    form.tags.length > 0 ||
    form.keywords !== INITIAL_PUBLISH_FORM.keywords ||
    form.dependencies.length > 0 ||
    form.kofi !== INITIAL_PUBLISH_FORM.kofi
  )
}
