import type { PublishFormState } from '@/components/developer/constants'
import { saveDeclaration, validateDeclaration } from '@/components/developer/declarationApi.ts'
import { formToValues } from '@/components/developer/formValues.ts'
import { findActionableSubmission } from '@/components/developer/sources.ts'
import type { FieldErrors as WireFieldErrors } from '@/components/developer/types.ts'

export type FieldErrors = Partial<Record<keyof PublishFormState, string[]>>

export type ValidateResult =
  | { status: 'valid' }
  | { status: 'invalid'; fields: FieldErrors; other: string[] }
  | { status: 'already_open'; id: number }
  | { status: 'not_open'; id: number }
  | { status: 'error'; message: string }

export type SubmitAddonResult =
  | { status: 'submitted'; id: number }
  | { status: 'already_open'; id: number }
  | { status: 'invalid'; fields: FieldErrors; other: string[] }
  | { status: 'not_open'; id: number }
  | { status: 'error'; message: string }

const NAME_ALREADY_TAKEN = 'name is already taken'

function isNameAlreadyTaken(
  submissionId: number | null,
  fields: { name?: string[] },
  other: string[]
): boolean {
  if (submissionId !== null) return false
  return [...(fields.name ?? []), ...other].some(
    message => message.trim().toLowerCase() === NAME_ALREADY_TAKEN
  )
}

export async function validateAddon(
  form: PublishFormState,
  submissionId: number | null = null
): Promise<ValidateResult> {
  const result = await validateDeclaration(formToValues(form), submissionId)
  if (result.status === 'valid') return { status: 'valid' }
  if (result.status === 'invalid') {
    if (isNameAlreadyTaken(submissionId, result.fields, result.other)) {
      const existing = await findActionableSubmission(form.name)
      if (existing.status === 'found') return { status: 'already_open', id: existing.id }
      if (existing.status === 'error') return existing
    }
    return { status: 'invalid', fields: toFormFieldErrors(result.fields), other: result.other }
  }
  if (result.status === 'not_open') return { status: 'not_open', id: result.id }
  return { status: 'error', message: result.message }
}

export async function submitAddon(
  form: PublishFormState,
  submissionId: number | null = null
): Promise<SubmitAddonResult> {
  if (submissionId === null) {
    const existing = await findActionableSubmission(form.name)
    if (existing.status === 'found') return { status: 'already_open', id: existing.id }
    if (existing.status === 'error') return existing
  }
  const result = await saveDeclaration(formToValues(form), submissionId)
  if (result.status === 'saved') return { status: 'submitted', id: result.id }
  if (result.status === 'already_open') return { status: 'already_open', id: result.id }
  if (result.status === 'invalid') {
    return { status: 'invalid', fields: toFormFieldErrors(result.fields), other: result.other }
  }
  if (result.status === 'not_open') return { status: 'not_open', id: result.id }
  return { status: 'error', message: result.message }
}

function toFormFieldErrors(fields: WireFieldErrors): FieldErrors {
  const next: FieldErrors = {}
  for (const [key, messages] of Object.entries(fields)) {
    next[key as keyof PublishFormState] = messages
  }
  return next
}
