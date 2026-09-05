import { INITIAL_PUBLISH_FORM, type PublishFormState } from '@/components/developer/constants'
import { parseSubmitAddonResponse } from '@/components/developer/submitParse'
import { apiClient } from '@/lib/api'

export interface AddonValidationError {
  field: string
  message: string
}

export type FieldErrors = Partial<Record<keyof PublishFormState, string[]>>

/** Explicit wire contract for validation endpoint, not PublishFormState. */
export type ValidateAddonPayload = {
  name: string
  alias: string
  description: string
  author: string
  repo: string
  branch: string
  tags: string[]
  keywords?: string[]
  dependencies?: string[]
  kofi?: string
}

export type ValidateResult =
  | { status: 'valid' }
  | { status: 'invalid'; fields: FieldErrors; other: string[] }
  | { status: 'error'; message: string }

export function publishFormToValidatePayload(form: PublishFormState): ValidateAddonPayload {
  const payload = {
    name: form.name,
    alias: form.alias,
    description: form.description,
    author: form.author,
    repo: form.repo,
    branch: form.branch,
    tags: form.tags,
  } satisfies ValidateAddonPayload

  const keywords = form.keywords.trim().split(/\s+/).filter(Boolean)
  const kofi = form.kofi.trim()

  return {
    ...payload,
    ...(keywords.length > 0 ? { keywords } : {}),
    ...(form.dependencies.length > 0 ? { dependencies: form.dependencies } : {}),
    ...(kofi ? { kofi } : {}),
  }
}

function groupValidationErrors(errors: AddonValidationError[]): {
  fields: FieldErrors
  other: string[]
} {
  const fields: FieldErrors = {}
  const other: string[] = []

  for (const error of errors) {
    if (Object.hasOwn(INITIAL_PUBLISH_FORM, error.field)) {
      const field = error.field as keyof PublishFormState
      fields[field] = [...(fields[field] ?? []), error.message]
      continue
    }
    other.push(error.message)
  }

  return { fields, other }
}

export type SubmitAddonResult =
  | { status: 'submitted'; prNumber: number; htmlUrl: string }
  | { status: 'already_open'; prNumber: number; htmlUrl: string }
  | { status: 'invalid'; fields: FieldErrors; other: string[] }
  | { status: 'error'; message: string }

export async function submitAddon(payload: ValidateAddonPayload): Promise<SubmitAddonResult> {
  const response = await apiClient.post('/addon/submit', payload)

  if (response.status === 401) {
    return { status: 'error', message: 'Sign in to publish an addon.' }
  }

  let body: unknown
  try {
    body = await response.json()
  } catch {
    return { status: 'error', message: 'Unexpected publish response.' }
  }

  const parsed = parseSubmitAddonResponse(response.status, body)
  if (parsed.status === 'invalid') {
    const grouped = groupValidationErrors(parsed.errors)
    return { status: 'invalid', fields: grouped.fields, other: grouped.other }
  }
  return parsed
}

export async function validateAddon(payload: ValidateAddonPayload): Promise<ValidateResult> {
  const response = await apiClient.post('/addon/validate', payload)

  if (response.status === 401) {
    return { status: 'error', message: 'Sign in to validate an addon.' }
  }

  if (!response.ok) {
    return {
      status: 'error',
      message: await errorMessage(response, "Couldn't validate this addon."),
    }
  }

  let body: unknown
  try {
    body = await response.json()
  } catch {
    return { status: 'error', message: 'Unexpected validation response.' }
  }

  if (body === null || typeof body !== 'object' || !('data' in body)) {
    return { status: 'error', message: 'Unexpected validation response.' }
  }

  const data = body.data
  if (
    data === null ||
    typeof data !== 'object' ||
    !('valid' in data) ||
    !('errors' in data) ||
    typeof data.valid !== 'boolean' ||
    !Array.isArray(data.errors)
  ) {
    return { status: 'error', message: 'Unexpected validation response.' }
  }

  if (data.valid) {
    return { status: 'valid' }
  }

  // Keep usable entries; ignore malformed ones. Empty usable set still means invalid.
  const usable = data.errors.filter((error): error is AddonValidationError => {
    if (!error || typeof error !== 'object') return false
    if (!('field' in error) || !('message' in error)) return false
    return typeof error.field === 'string' && typeof error.message === 'string'
  })
  const grouped = groupValidationErrors(usable)
  return { status: 'invalid', fields: grouped.fields, other: grouped.other }
}

async function errorMessage(response: Response, fallback: string): Promise<string> {
  try {
    const body = (await response.json()) as { message?: string }
    if (typeof body.message === 'string' && body.message.trim()) return body.message
  } catch {
    // keep fallback
  }
  return fallback
}
