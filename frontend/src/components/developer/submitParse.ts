export type SubmitValidationError = { field: string; message: string }

export type ParsedSubmitAddon =
  | { status: 'submitted'; prNumber: number; htmlUrl: string }
  | { status: 'already_open'; prNumber: number; htmlUrl: string }
  | { status: 'invalid'; errors: SubmitValidationError[] }
  | { status: 'error'; message: string }

export function parseSubmitAddonResponse(statusCode: number, body: unknown): ParsedSubmitAddon {
  if (statusCode === 401) {
    return { status: 'error', message: 'Sign in to publish an addon.' }
  }

  if (
    statusCode === 409 &&
    body !== null &&
    typeof body === 'object' &&
    'pr_number' in body &&
    'html_url' in body &&
    typeof body.pr_number === 'number' &&
    typeof body.html_url === 'string'
  ) {
    return { status: 'already_open', prNumber: body.pr_number, htmlUrl: body.html_url }
  }

  if (
    statusCode === 400 &&
    body !== null &&
    typeof body === 'object' &&
    'valid' in body &&
    'errors' in body &&
    body.valid === false &&
    Array.isArray(body.errors)
  ) {
    const errors: SubmitValidationError[] = []
    for (const error of body.errors) {
      if (!error || typeof error !== 'object') continue
      if (!('field' in error) || !('message' in error)) continue
      if (typeof error.field !== 'string' || typeof error.message !== 'string') continue
      errors.push({ field: error.field, message: error.message })
    }
    return { status: 'invalid', errors }
  }

  if (statusCode === 200) {
    if (
      body !== null &&
      typeof body === 'object' &&
      'status' in body &&
      'data' in body &&
      body.status === true &&
      body.data !== null &&
      typeof body.data === 'object' &&
      'pr_number' in body.data &&
      'html_url' in body.data &&
      'status' in body.data &&
      typeof body.data.pr_number === 'number' &&
      typeof body.data.html_url === 'string' &&
      typeof body.data.status === 'string'
    ) {
      return {
        status: 'submitted',
        prNumber: body.data.pr_number,
        htmlUrl: body.data.html_url,
      }
    }
    return { status: 'error', message: 'Unexpected publish response.' }
  }

  if (
    body !== null &&
    typeof body === 'object' &&
    'message' in body &&
    typeof body.message === 'string' &&
    body.message !== ''
  ) {
    return { status: 'error', message: body.message }
  }
  return { status: 'error', message: "Couldn't publish this addon." }
}
