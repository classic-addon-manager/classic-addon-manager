import {
  parseSaveResponse,
  type ParseSaveResult,
  parseValidateResponse,
  type ParseValidateResult,
  parseWithdrawResponse,
  type ParseWithdrawResult,
} from '@/components/developer/parse.ts'
import type { DeclarationValues } from '@/components/developer/types.ts'
import { v1SchemaKeys } from '@/components/developer/types.ts'
import { apiClient } from '@/lib/api'

export type { ParseSaveResult, ParseValidateResult, ParseWithdrawResult }

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

export async function validateDeclaration(
  values: DeclarationValues,
  submissionId: number | null
): Promise<ParseValidateResult> {
  const path = editorValidatePath(submissionId)
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
    const save = editorSaveRequest(submissionId)
    const response =
      save.method === 'POST'
        ? await apiClient.post(save.path, values)
        : await apiClient.put(save.path, values)
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

export async function withdrawDeclaration(submissionId: number): Promise<ParseWithdrawResult> {
  try {
    const response = await apiClient.post(editorWithdrawPath(submissionId))
    let body: unknown
    try {
      body = await response.json()
    } catch {
      if (response.status === 401) {
        return { status: 'unauthorized', message: 'Sign in to withdraw a submission.' }
      }
      return { status: 'error', message: 'Unexpected withdraw response.' }
    }
    return parseWithdrawResponse(response.status, body)
  } catch {
    return { status: 'error', message: 'Unexpected withdraw response.' }
  }
}
