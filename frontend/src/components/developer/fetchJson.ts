import { apiClient } from '@/lib/api'

type FetchJsonInit = { method?: 'GET' | 'POST' | 'PUT'; body?: unknown }

export async function fetchJson(
  path: string,
  init: FetchJsonInit = {}
): Promise<{
  status: number
  body: unknown
}> {
  const response =
    init.method === 'POST'
      ? init.body instanceof FormData
        ? await apiClient.postForm(path, init.body)
        : await apiClient.post(path, init.body)
      : init.method === 'PUT'
        ? await apiClient.put(path, init.body)
        : await apiClient.get(path)
  let body: unknown
  try {
    body = await response.json()
  } catch {
    body = undefined
  }
  return { status: response.status, body }
}

type Failure = { status: 'unauthorized' | 'error'; message: string }

export async function fetchParsed<TResult>(
  path: string,
  init: FetchJsonInit,
  parse: (status: number, body: unknown) => TResult,
  unauthorizedMessage: string,
  errorMessage: string
): Promise<TResult | Failure> {
  try {
    const { status, body } = await fetchJson(path, init)
    if (body === undefined) {
      return status === 401
        ? { status: 'unauthorized', message: unauthorizedMessage }
        : { status: 'error', message: errorMessage }
    }
    return parse(status, body)
  } catch {
    return { status: 'error', message: errorMessage }
  }
}
