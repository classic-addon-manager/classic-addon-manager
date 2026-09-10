import {
  parseSaveResponse,
  type ParseSaveResult,
  parseValidateResponse,
  type ParseValidateResult,
} from '@/components/developer/parse.ts'
import type { DeclarationValues } from '@/components/developer/types.ts'
import { v1SchemaKeys } from '@/components/developer/types.ts'
import { apiClient } from '@/lib/api'

export type { ParseSaveResult, ParseValidateResult }

export async function validateDeclaration(
  values: DeclarationValues,
  submissionId: number | null
): Promise<ParseValidateResult> {
  const path =
    submissionId === null
      ? '/dev/addon/validate'
      : `/dev/addon/submissions/${submissionId}/validate`
  try {
    const response = await apiClient.post(path, values)
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to validate an addon.' }
      }
      return { status: 'error', message: 'Unexpected validation response.' }
    }
    return parseValidateResponse(response.status, body, v1SchemaKeys())
  } catch {
    return { status: 'error', message: 'Unexpected validation response.' }
  }
}

export async function saveDeclaration(
  values: DeclarationValues,
  submissionId: number | null
): Promise<ParseSaveResult> {
  try {
    const response =
      submissionId === null
        ? await apiClient.post('/dev/addon/submit', values)
        : await apiClient.put(`/dev/addon/submissions/${submissionId}`, values)
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to submit an addon.' }
      }
      return { status: 'error', message: 'Unexpected save response.' }
    }
    return parseSaveResponse(response.status, body, v1SchemaKeys())
  } catch {
    return { status: 'error', message: 'Unexpected save response.' }
  }
}
