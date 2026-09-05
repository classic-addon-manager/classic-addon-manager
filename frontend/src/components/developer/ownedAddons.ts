import {
  type OwnedAddon,
  type OwnedSubmission,
  parseOwnedAddonsResponse,
} from '@/components/developer/ownedParse'
import { apiClient } from '@/lib/api'

export type GetOwnedAddonsResult =
  | { status: 'ok'; addons: OwnedAddon[]; submissions: OwnedSubmission[] }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

export async function getOwnedAddons(): Promise<GetOwnedAddonsResult> {
  try {
    const response = await apiClient.get('/me/owned-addons')

    if (response.status === 401) {
      return { status: 'unauthorized', message: 'Sign in to view your addons.' }
    }

    if (response.status !== 200) {
      return {
        status: 'error',
        message: await errorMessage(response, "Couldn't load your addons."),
      }
    }

    let body: unknown
    try {
      body = await response.json()
    } catch {
      return { status: 'error', message: 'Unexpected owned addons response.' }
    }

    return parseOwnedAddonsResponse(200, body)
  } catch {
    return { status: 'error', message: "Couldn't load your addons." }
  }
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
