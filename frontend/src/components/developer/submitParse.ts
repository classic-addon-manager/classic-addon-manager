export type SubmitValidationError = { field: string; message: string }

export type ParsedSubmitAddon =
  | { status: 'submitted'; id: number }
  | { status: 'already_open'; id: number }
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
    'status' in body &&
    body.status === false &&
    'id' in body &&
    typeof body.id === 'number' &&
    Number.isFinite(body.id)
  ) {
    return { status: 'already_open', id: body.id }
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
      'id' in body.data &&
      'status' in body.data &&
      typeof body.data.id === 'number' &&
      Number.isFinite(body.data.id) &&
      body.data.status === 'open'
    ) {
      return { status: 'submitted', id: body.data.id }
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
