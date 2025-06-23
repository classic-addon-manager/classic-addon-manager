import { useUserStore } from '@/stores/userStore.ts'

const API_URL = 'https://aac.gaijin.dev'

function createHeaders(version: string, token: string): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    'X-Client': version,
    'X-Token': token,
  }
}

export const createApiClient = (version: string) => ({
  async get(url: string): Promise<Response> {
    return fetch(API_URL + url, {
      method: 'GET',
      headers: createHeaders(version, useUserStore.getState().token),
    })
  },

  async post(url: string, data: unknown): Promise<Response> {
    return fetch(API_URL + url, {
      method: 'POST',
      headers: createHeaders(version, useUserStore.getState().token),
      body: JSON.stringify(data),
    })
  },
})
