import {
  parseSaveResponse,
  type ParseSaveResult,
  parseValidateResponse,
  type ParseValidateResult,
  parseWithdrawResponse,
  type ParseWithdrawResult,
} from '@/components/developer/parse.ts'
import type { DeclarationValues } from '@/components/developer/types.ts'
import { SCHEMA_FIELD_KEYS } from '@/components/developer/types.ts'

import { fetchJson, fetchParsed } from './fetchJson.ts'

export type { ParseSaveResult, ParseValidateResult, ParseWithdrawResult }

export type UploadIconResult =
  | { status: 'uploaded'; assetId: string; url: string }
  | { status: 'error'; message: string }

export async function uploadIcon(file: File): Promise<UploadIconResult> {
  const form = new FormData()
  form.append('icon', file)
  try {
    const { status, body } = await fetchJson('/dev/addon/icon', { method: 'POST', body: form })
    if (
      status >= 200 &&
      status < 300 &&
      typeof body === 'object' &&
      body !== null &&
      'data' in body &&
      typeof body.data === 'object' &&
      body.data !== null &&
      'asset_id' in body.data &&
      typeof body.data.asset_id === 'string' &&
      body.data.asset_id !== '' &&
      'url' in body.data &&
      typeof body.data.url === 'string'
    ) {
      return { status: 'uploaded', assetId: body.data.asset_id, url: body.data.url }
    }
    if (typeof body === 'object' && body !== null && 'data' in body) {
      const data = body.data
      if (
        typeof data === 'object' &&
        data !== null &&
        'field' in data &&
        data.field === 'icon' &&
        'message' in data &&
        typeof data.message === 'string'
      ) {
        return { status: 'error', message: data.message }
      }
    }
    return { status: 'error', message: 'Icon upload failed.' }
  } catch {
    return { status: 'error', message: 'Icon upload failed.' }
  }
}

export function editorValidatePath(submissionId: number | null): string {
  return submissionId === null
    ? '/dev/addon/validate'
    : `/dev/addon/submissions/${submissionId}/validate`
}

export function editorSaveRequest(submissionId: number | null): {
  method: 'POST' | 'PUT'
  path: string
} {
  return submissionId === null
    ? { method: 'POST', path: '/dev/addon/submit' }
    : { method: 'PUT', path: `/dev/addon/submissions/${submissionId}` }
}

export function editorWithdrawPath(submissionId: number): string {
  return `/dev/addon/submissions/${submissionId}/withdraw`
}

function declarationErrorKeys(): Set<string> {
  return new Set<string>([...SCHEMA_FIELD_KEYS, 'icon'])
}

export async function validateDeclaration(
  values: DeclarationValues,
  submissionId: number | null
): Promise<ParseValidateResult> {
  const path = editorValidatePath(submissionId)
  return fetchParsed(
    path,
    { method: 'POST', body: values },
    (status, body) => parseValidateResponse(status, body, declarationErrorKeys()),
    'Sign in to validate an addon.',
    'Unexpected validation response.'
  )
}

export async function saveDeclaration(
  values: DeclarationValues,
  submissionId: number | null
): Promise<ParseSaveResult> {
  const save = editorSaveRequest(submissionId)
  return fetchParsed(
    save.path,
    { method: save.method, body: values },
    (status, body) => parseSaveResponse(status, body, declarationErrorKeys()),
    'Sign in to submit an addon.',
    'Unexpected save response.'
  )
}

export async function withdrawDeclaration(submissionId: number): Promise<ParseWithdrawResult> {
  return fetchParsed(
    editorWithdrawPath(submissionId),
    { method: 'POST' },
    parseWithdrawResponse,
    'Sign in to withdraw a submission.',
    'Unexpected withdraw response.'
  )
}
