export type OwnedAddon = {
  name: string
  alias: string
  repo: string
  branch: string | null
  author: string
  description: string
  tags: string[]
  downloads: number
  likePercentage: number | null
  warning: string | null
  addedAt: string | null
}

export type OwnedSubmissionMessage = {
  id: number
  body: string
  createdAt: string | null
}

export type OwnedSubmissionPayload = {
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
}

export type OwnedSubmission = {
  id: number
  status: 'open' | 'rejected'
  title: string
  createdAt: string | null
  payload: OwnedSubmissionPayload
  messages: OwnedSubmissionMessage[]
}

export type ParsedOwnedAddons =
  | { status: 'ok'; addons: OwnedAddon[]; submissions: OwnedSubmission[] }
  | { status: 'error'; message: string }

const UNEXPECTED = 'Unexpected owned addons response.'

export function isFiniteDateString(value: string): boolean {
  return Number.isFinite(new Date(value).getTime())
}

export function parseOwnedAddonsResponse(statusCode: number, body: unknown): ParsedOwnedAddons {
  if (statusCode !== 200) {
    return { status: 'error', message: UNEXPECTED }
  }

  if (body === null || typeof body !== 'object') {
    return { status: 'error', message: UNEXPECTED }
  }
  if (!('status' in body) || body.status !== true) {
    return { status: 'error', message: UNEXPECTED }
  }
  if (!('data' in body) || body.data === null || typeof body.data !== 'object') {
    return { status: 'error', message: UNEXPECTED }
  }

  const data = body.data
  if (!('addons' in data) || !Array.isArray(data.addons)) {
    return { status: 'error', message: UNEXPECTED }
  }
  if (!('submissions' in data) || !Array.isArray(data.submissions)) {
    return { status: 'error', message: UNEXPECTED }
  }

  const addons: OwnedAddon[] = []
  for (const item of data.addons) {
    const addon = parseAddon(item)
    if (addon) addons.push(addon)
  }

  const submissions: OwnedSubmission[] = []
  for (const item of data.submissions) {
    const submission = parseSubmission(item)
    if (submission) submissions.push(submission)
  }

  return { status: 'ok', addons, submissions }
}

function parseAddon(value: unknown): OwnedAddon | null {
  if (value === null || typeof value !== 'object') return null
  if (!('name' in value) || typeof value.name !== 'string' || value.name === '') return null

  const alias =
    'alias' in value && typeof value.alias === 'string' && value.alias !== ''
      ? value.alias
      : value.name

  const repo = 'repo' in value && typeof value.repo === 'string' ? value.repo : ''
  const branch =
    'branch' in value && typeof value.branch === 'string' && value.branch !== ''
      ? value.branch
      : null
  const author = 'author' in value && typeof value.author === 'string' ? value.author : ''
  const description =
    'description' in value && typeof value.description === 'string' ? value.description : ''
  const tags = 'tags' in value && isStringArray(value.tags) ? value.tags : []
  const downloads =
    'downloads' in value && typeof value.downloads === 'number' && Number.isFinite(value.downloads)
      ? value.downloads
      : 0
  const likePercentage =
    'like_percentage' in value && typeof value.like_percentage === 'number'
      ? value.like_percentage
      : null
  const warning =
    'warning' in value && typeof value.warning === 'string' && value.warning !== ''
      ? value.warning
      : null
  const addedAt =
    'added_at' in value && typeof value.added_at === 'string' && isFiniteDateString(value.added_at)
      ? value.added_at
      : null

  return {
    name: value.name,
    alias,
    repo,
    branch,
    author,
    description,
    tags,
    downloads,
    likePercentage,
    warning,
    addedAt,
  }
}

function parseSubmission(value: unknown): OwnedSubmission | null {
  if (value === null || typeof value !== 'object') return null
  if (!('id' in value) || typeof value.id !== 'number' || !Number.isFinite(value.id)) {
    return null
  }
  if (!('status' in value) || (value.status !== 'open' && value.status !== 'rejected')) {
    return null
  }
  if (!('payload' in value) || value.payload === null || typeof value.payload !== 'object') {
    return null
  }

  const raw = value.payload
  if (!('name' in raw) || typeof raw.name !== 'string' || raw.name === '') {
    return null
  }

  const payload: OwnedSubmissionPayload = {
    name: raw.name,
    alias: 'alias' in raw && typeof raw.alias === 'string' ? raw.alias : '',
    description: 'description' in raw && typeof raw.description === 'string' ? raw.description : '',
    author: 'author' in raw && typeof raw.author === 'string' ? raw.author : '',
    repo: 'repo' in raw && typeof raw.repo === 'string' ? raw.repo : '',
    branch: 'branch' in raw && typeof raw.branch === 'string' ? raw.branch : '',
    tags: 'tags' in raw && isStringArray(raw.tags) ? raw.tags : [],
    keywords: 'keywords' in raw && isStringArray(raw.keywords) ? raw.keywords : [],
    dependencies: 'dependencies' in raw && isStringArray(raw.dependencies) ? raw.dependencies : [],
    kofi: 'kofi' in raw && typeof raw.kofi === 'string' ? raw.kofi : '',
  }

  const createdAt =
    'created_at' in value &&
    typeof value.created_at === 'string' &&
    isFiniteDateString(value.created_at)
      ? value.created_at
      : null

  return {
    id: value.id,
    status: value.status,
    title: payload.alias !== '' ? payload.alias : payload.name,
    createdAt,
    payload,
    messages: parseMessages(value),
  }
}

function parseMessages(value: object): OwnedSubmissionMessage[] {
  if (!('messages' in value) || !Array.isArray(value.messages)) return []
  const messages: OwnedSubmissionMessage[] = []
  for (const item of value.messages) {
    if (item === null || typeof item !== 'object') continue
    if (!('id' in item) || typeof item.id !== 'number' || !Number.isFinite(item.id)) continue
    if (!('body' in item) || typeof item.body !== 'string') continue
    const createdAt =
      'created_at' in item &&
      typeof item.created_at === 'string' &&
      isFiniteDateString(item.created_at)
        ? item.created_at
        : null
    messages.push({ id: item.id, body: item.body, createdAt })
  }
  return messages
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(item => typeof item === 'string')
}
