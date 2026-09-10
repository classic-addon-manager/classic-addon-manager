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

  const envelope = parseEnvelope(body)

  if (statusCode === 409 && envelope?.message === 'submission already open') {
    const id = dataId(envelope)
    if (id !== null) return { status: 'already_open', id }
  }

  if (
    statusCode === 400 &&
    envelope !== null &&
    envelope.status === false &&
    envelope.message === 'invalid addon'
  ) {
    const data = envelope.data
    if (
      data !== null &&
      typeof data === 'object' &&
      'valid' in data &&
      data.valid === false &&
      'errors' in data &&
      Array.isArray(data.errors)
    ) {
      const errors: SubmitValidationError[] = []
      for (const error of data.errors) {
        if (!error || typeof error !== 'object') continue
        if (!('field' in error) || !('message' in error)) continue
        if (typeof error.field !== 'string' || typeof error.message !== 'string') continue
        errors.push({ field: error.field, message: error.message })
      }
      return { status: 'invalid', errors }
    }
  }

  if (statusCode === 200) {
    if (
      envelope !== null &&
      envelope.status === true &&
      envelope.data !== null &&
      typeof envelope.data === 'object' &&
      'id' in envelope.data &&
      typeof envelope.data.id === 'number' &&
      Number.isFinite(envelope.data.id)
    ) {
      return { status: 'submitted', id: envelope.data.id }
    }
    return { status: 'error', message: 'Unexpected publish response.' }
  }

  if (envelope !== null && envelope.message !== '') {
    return { status: 'error', message: envelope.message }
  }
  return { status: 'error', message: "Couldn't publish this addon." }
}

function parseEnvelope(body: unknown): { status: boolean; message: string; data: unknown } | null {
  if (body === null || typeof body !== 'object') return null
  if (!('status' in body) || !('message' in body) || !('data' in body)) return null
  if (typeof body.status !== 'boolean' || typeof body.message !== 'string') return null
  return { status: body.status, message: body.message, data: body.data }
}

function dataId(envelope: { data: unknown }): number | null {
  if (envelope.data === null || typeof envelope.data !== 'object') return null
  if (!('id' in envelope.data) || typeof envelope.data.id !== 'number') return null
  if (!Number.isFinite(envelope.data.id)) return null
  return envelope.data.id
}
