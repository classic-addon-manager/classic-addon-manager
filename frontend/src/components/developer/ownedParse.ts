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

export type OwnedSubmission = {
  prNumber: number
  status: 'open' | 'closed'
  htmlUrl: string
  repo: string | null
  title: string
  createdAt: string | null
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
  if (
    !('pr_number' in value) ||
    typeof value.pr_number !== 'number' ||
    !Number.isFinite(value.pr_number)
  ) {
    return null
  }

  const status = 'status' in value && value.status === 'open' ? 'open' : 'closed'
  const htmlUrl = 'html_url' in value && typeof value.html_url === 'string' ? value.html_url : ''

  const payload =
    'payload' in value && value.payload !== null && typeof value.payload === 'object'
      ? value.payload
      : null

  const payloadAlias =
    payload !== null &&
    'alias' in payload &&
    typeof payload.alias === 'string' &&
    payload.alias !== ''
      ? payload.alias
      : null
  const payloadName =
    payload !== null && 'name' in payload && typeof payload.name === 'string' && payload.name !== ''
      ? payload.name
      : null
  const title = payloadAlias ?? payloadName ?? `PR #${value.pr_number}`

  const repo =
    payload !== null && 'repo' in payload && typeof payload.repo === 'string' && payload.repo !== ''
      ? payload.repo
      : null

  const createdAt =
    'created_at' in value &&
    typeof value.created_at === 'string' &&
    isFiniteDateString(value.created_at)
      ? value.created_at
      : null

  return {
    prNumber: value.pr_number,
    status,
    htmlUrl,
    repo,
    title,
    createdAt,
  }
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(item => typeof item === 'string')
}
