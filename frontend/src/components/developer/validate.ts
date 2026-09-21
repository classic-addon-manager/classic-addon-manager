import { imageSize } from 'image-size'

import type { PublishFormState } from '@/components/developer/constants'
import { saveDeclaration, validateDeclaration } from '@/components/developer/declarationApi.ts'
import { formToValues, textOf } from '@/components/developer/formValues.ts'
import { findActionableSubmission } from '@/components/developer/sources.ts'

export type FieldErrors = Partial<Record<string, string[]>>

// Client-side icon rules. UX only: the server re-checks everything once icon
// transport exists: keep both in sync when that lands.
const ICON_MAX_BYTES = 3 * 1024 * 1024
const ICON_MIN_DIMENSION = 50

export async function validateIconFile(file: File): Promise<string[]> {
  const errors: string[] = []
  const claimsPng = file.type === 'image/png' && file.name.toLowerCase().endsWith('.png')
  if (!claimsPng) {
    errors.push('Icon must be a PNG file.')
  }
  if (file.size > ICON_MAX_BYTES) {
    errors.push('Icon must be 3 MB or smaller.')
  }
  const dimensions = await readImageDimensions(file)
  if (dimensions === null) {
    if (claimsPng) errors.push('Icon must be a valid PNG file.')
  } else {
    const { width, height } = dimensions
    if (width !== height) errors.push('Icon must be square (1:1).')
    if (width < ICON_MIN_DIMENSION || height < ICON_MIN_DIMENSION) {
      errors.push(`Icon must be at least ${ICON_MIN_DIMENSION}×${ICON_MIN_DIMENSION} pixels.`)
    }
  }
  return errors
}

async function readImageDimensions(file: File): Promise<{ width: number; height: number } | null> {
  try {
    // image-size detects the type from the file signature, so a file renamed
    // to .png but holding JPEG/WebP data is rejected here.
    const bytes = new Uint8Array(await file.slice(0, 512 * 1024).arrayBuffer())
    const { width, height, type } = imageSize(bytes)
    if (type !== 'png') return null
    return { width, height }
  } catch {
    return null
  }
}

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
      const existing = await findActionableSubmission(textOf(form.values, 'name'))
      if (existing.status === 'found') return { status: 'already_open', id: existing.id }
      if (existing.status === 'error') return existing
    }
    return { status: 'invalid', fields: result.fields, other: result.other }
  }
  if (result.status === 'not_open') return { status: 'not_open', id: result.id }
  return { status: 'error', message: result.message }
}

export async function submitAddon(
  form: PublishFormState,
  submissionId: number | null = null
): Promise<SubmitAddonResult> {
  if (submissionId === null) {
    const existing = await findActionableSubmission(textOf(form.values, 'name'))
    if (existing.status === 'found') return { status: 'already_open', id: existing.id }
    if (existing.status === 'error') return existing
  }
  const result = await saveDeclaration(formToValues(form), submissionId)
  if (result.status === 'saved') return { status: 'submitted', id: result.id }
  if (result.status === 'already_open') return { status: 'already_open', id: result.id }
  if (result.status === 'invalid') {
    return { status: 'invalid', fields: result.fields, other: result.other }
  }
  if (result.status === 'not_open') return { status: 'not_open', id: result.id }
  return { status: 'error', message: result.message }
}
