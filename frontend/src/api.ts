import {getVersion} from '$atoms/application.svelte'
import {getToken} from '$atoms/user.svelte'

const API_URL = 'https://aac.gaijin.dev'

function createHeaders(): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
    'X-Client': getVersion(),
    'X-Token': getToken()
  }
}

export const apiClient = {
  get,
  post
}

async function get(url: string): Promise<Response> {
  return fetch(API_URL + url, {
    method: 'GET',
    headers: createHeaders()
  })
}

async function post(url: string, data: unknown): Promise<Response> {
  return fetch(API_URL + url, {
    method: 'POST',
    headers: createHeaders(),
    body: JSON.stringify(data)
  })
}