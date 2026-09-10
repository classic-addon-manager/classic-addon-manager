import type {
  AddonSchema,
  AddonSources,
  DeclarationKind,
  DeclarationValues,
  FieldErrors,
  SchemaField,
  SourceAddon,
  SourceSubmission,
  Widget,
  WireValue,
} from './types.ts'

export type Envelope = { status: boolean; message: string; data: unknown }

export type ParseSchemaResult =
  | { status: 'ok'; schema: AddonSchema }
  | { status: 'unsupported'; message: string }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

export type ParseSourcesResult =
  | { status: 'ok'; sources: AddonSources }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

export type ParseValuesResult =
  | { status: 'ok'; kind: DeclarationKind; values: DeclarationValues }
  | { status: 'not_found'; message: string }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

export type ParseValidateResult =
  | { status: 'valid' }
  | { status: 'invalid'; fields: FieldErrors; other: string[] }
  | { status: 'unauthorized'; message: string }
  | { status: 'not_found'; message: string }
  | { status: 'not_open'; id: number }
  | { status: 'error'; message: string }

export type ParseSaveResult =
  | { status: 'saved'; id: number; revision: number }
  | { status: 'invalid'; fields: FieldErrors; other: string[] }
  | { status: 'already_open'; id: number }
  | { status: 'not_open'; id: number }
  | { status: 'unauthorized'; message: string }
  | { status: 'not_found'; message: string }
  | { status: 'error'; message: string }

const WIDGETS: Record<string, true> = {
  text: true,
  textarea: true,
  'enum-multi': true,
  'string-list': true,
}
const UNSUPPORTED_SCHEMA = 'This app needs an update to edit addon declarations.'
const SOURCE_UNAVAILABLE = 'This source is unavailable.'

export function parseEnvelope(body: unknown): Envelope | null {
  if (body === null || typeof body !== 'object') return null
  if (!('status' in body) || !('message' in body) || !('data' in body)) return null
  if (typeof body.status !== 'boolean' || typeof body.message !== 'string') return null
  return { status: body.status, message: body.message, data: body.data }
}

export function groupValidationErrors(
  errors: { field: string; message: string }[],
  schemaKeys: Set<string>
): { fields: FieldErrors; other: string[] } {
  const fields: FieldErrors = {}
  const other: string[] = []
  for (const error of errors) {
    if (schemaKeys.has(error.field)) {
      fields[error.field] = [...(fields[error.field] ?? []), error.message]
      continue
    }
    other.push(error.message)
  }
  return { fields, other }
}

export function parseAddonSchema(statusCode: number, body: unknown): ParseSchemaResult {
  const envelope = parseEnvelope(body)
  if (statusCode === 401) {
    return { status: 'unauthorized', message: 'Sign in to edit addon declarations.' }
  }
  if (statusCode !== 200 || envelope === null || envelope.status !== true) {
    return { status: 'error', message: envelope?.message || 'Unexpected schema response.' }
  }
  const data = envelope.data
  if (data === null || typeof data !== 'object') {
    return { status: 'error', message: 'Unexpected schema response.' }
  }
  if (!('version' in data) || typeof data.version !== 'number') {
    return { status: 'error', message: 'Unexpected schema response.' }
  }
  if (data.version !== 1) {
    return { status: 'unsupported', message: UNSUPPORTED_SCHEMA }
  }
  if (
    !('widgets' in data) ||
    !Array.isArray(data.widgets) ||
    !('fields' in data) ||
    !Array.isArray(data.fields)
  ) {
    return { status: 'error', message: 'Unexpected schema response.' }
  }
  if (!data.widgets.every(isWidget)) {
    return { status: 'unsupported', message: UNSUPPORTED_SCHEMA }
  }
  const fields: SchemaField[] = []
  for (const item of data.fields) {
    const parsed = parseSchemaField(item)
    if (parsed === 'unsupported') return { status: 'unsupported', message: UNSUPPORTED_SCHEMA }
    if (parsed === null) return { status: 'error', message: 'Unexpected schema response.' }
    fields.push(parsed)
  }
  return { status: 'ok', schema: { version: 1, widgets: data.widgets, fields } }
}

export function parseAddonSources(statusCode: number, body: unknown): ParseSourcesResult {
  const envelope = parseEnvelope(body)
  if (statusCode === 401) {
    return { status: 'unauthorized', message: 'Sign in to view your addons.' }
  }
  if (statusCode !== 200 || envelope === null || envelope.status !== true) {
    return { status: 'error', message: envelope?.message || 'Unexpected sources response.' }
  }
  const data = envelope.data
  if (data === null || typeof data !== 'object') {
    return { status: 'error', message: 'Unexpected sources response.' }
  }
  if (
    !('addons' in data) ||
    !Array.isArray(data.addons) ||
    !('submissions' in data) ||
    !Array.isArray(data.submissions)
  ) {
    return { status: 'error', message: 'Unexpected sources response.' }
  }
  const addons: SourceAddon[] = []
  for (const item of data.addons) {
    const addon = parseSourceAddon(item)
    if (addon) addons.push(addon)
  }
  const submissions: SourceSubmission[] = []
  for (const item of data.submissions) {
    const submission = parseSourceSubmission(item)
    if (submission) submissions.push(submission)
  }
  return { status: 'ok', sources: { addons, submissions } }
}

export function parseAddonValues(
  statusCode: number,
  body: unknown,
  schema: AddonSchema
): ParseValuesResult {
  const envelope = parseEnvelope(body)
  if (statusCode === 401) {
    return { status: 'unauthorized', message: 'Sign in to load addon values.' }
  }
  if (statusCode === 400 && envelope?.message === 'invalid values query') {
    return { status: 'not_found', message: SOURCE_UNAVAILABLE }
  }
  if (statusCode === 404 && envelope?.message === 'not found') {
    return { status: 'not_found', message: SOURCE_UNAVAILABLE }
  }
  if (statusCode !== 200 || envelope === null || envelope.status !== true) {
    return { status: 'error', message: envelope?.message || 'Unexpected values response.' }
  }
  const data = envelope.data
  if (data === null || typeof data !== 'object') {
    return { status: 'error', message: 'Unexpected values response.' }
  }
  if (!('kind' in data) || (data.kind !== 'new' && data.kind !== 'update')) {
    return { status: 'error', message: 'Unexpected values response.' }
  }
  if (
    !('values' in data) ||
    data.values === null ||
    typeof data.values !== 'object' ||
    Array.isArray(data.values)
  ) {
    return { status: 'error', message: 'Unexpected values response.' }
  }
  const raw = data.values as Record<string, unknown>
  const values: DeclarationValues = {}
  for (const field of schema.fields) {
    const mapped = mapWireValue(field, raw[field.key])
    if (mapped === 'error') return { status: 'error', message: 'Unexpected values response.' }
    values[field.key] = mapped
  }
  return { status: 'ok', kind: data.kind, values }
}

export function parseValidateResponse(
  statusCode: number,
  body: unknown,
  schemaKeys: Set<string>
): ParseValidateResult {
  const envelope = parseEnvelope(body)
  if (statusCode === 401) {
    return { status: 'unauthorized', message: 'Sign in to validate an addon.' }
  }
  const closed = closedConflict(statusCode, envelope)
  if (closed) return closed
  if (isNotFound(statusCode, envelope)) {
    return { status: 'not_found', message: SOURCE_UNAVAILABLE }
  }
  if (statusCode !== 200 || envelope === null || envelope.status !== true) {
    return { status: 'error', message: envelope?.message || 'Unexpected validation response.' }
  }
  const data = envelope.data
  if (
    data === null ||
    typeof data !== 'object' ||
    !('valid' in data) ||
    typeof data.valid !== 'boolean'
  ) {
    return { status: 'error', message: 'Unexpected validation response.' }
  }
  if (data.valid) return { status: 'valid' }
  return { status: 'invalid', ...groupValidationErrors(parseErrorItems(data), schemaKeys) }
}

export function parseSaveResponse(
  statusCode: number,
  body: unknown,
  schemaKeys: Set<string>
): ParseSaveResult {
  const envelope = parseEnvelope(body)
  if (statusCode === 401) {
    return { status: 'unauthorized', message: 'Sign in to submit an addon.' }
  }
  if (statusCode === 409 && envelope?.message === 'submission already open') {
    const id = dataId(envelope)
    if (id !== null) return { status: 'already_open', id }
  }
  const closed = closedConflict(statusCode, envelope)
  if (closed) return closed
  if (isNotFound(statusCode, envelope)) {
    return { status: 'not_found', message: SOURCE_UNAVAILABLE }
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
      return { status: 'invalid', ...groupValidationErrors(parseErrorItems(data), schemaKeys) }
    }
  }
  if (statusCode !== 200 || envelope === null || envelope.status !== true) {
    return { status: 'error', message: envelope?.message || 'Unexpected save response.' }
  }
  const data = envelope.data
  if (
    data === null ||
    typeof data !== 'object' ||
    !('id' in data) ||
    typeof data.id !== 'number' ||
    !Number.isFinite(data.id) ||
    !('revision' in data) ||
    typeof data.revision !== 'number' ||
    !Number.isFinite(data.revision)
  ) {
    return { status: 'error', message: 'Unexpected save response.' }
  }
  return { status: 'saved', id: data.id, revision: data.revision }
}

function isWidget(value: unknown): value is Widget {
  return typeof value === 'string' && WIDGETS[value] === true
}

function parseSchemaField(value: unknown): SchemaField | 'unsupported' | null {
  if (value === null || typeof value !== 'object') return null
  if (!('key' in value) || typeof value.key !== 'string' || value.key === '') return null
  if (!('label' in value) || typeof value.label !== 'string') return null
  if (!('section' in value) || typeof value.section !== 'string') return null
  if (!('widget' in value) || typeof value.widget !== 'string') return null
  if (!isWidget(value.widget)) return 'unsupported'
  if (!('required' in value) || typeof value.required !== 'boolean') return null
  const field: SchemaField = {
    key: value.key,
    label: value.label,
    section: value.section,
    widget: value.widget,
    required: value.required,
  }
  if ('hint' in value && value.hint !== undefined) {
    if (typeof value.hint !== 'string') return null
    field.hint = value.hint
  }
  if ('immutable' in value && value.immutable !== undefined) {
    if (typeof value.immutable !== 'boolean') return null
    field.immutable = value.immutable
  }
  if ('pattern' in value && value.pattern !== undefined) {
    if (typeof value.pattern !== 'string') return null
    field.pattern = value.pattern
  }
  if ('maxLength' in value && value.maxLength !== undefined) {
    if (typeof value.maxLength !== 'number' || !Number.isFinite(value.maxLength)) return null
    field.maxLength = value.maxLength
  }
  if ('lengthUnit' in value && value.lengthUnit !== undefined) {
    if (value.lengthUnit !== 'runes' && value.lengthUnit !== 'bytes') return null
    field.lengthUnit = value.lengthUnit
  }
  if ('enum' in value && value.enum !== undefined) {
    if (!Array.isArray(value.enum) || !value.enum.every(item => typeof item === 'string'))
      return null
    field.enum = value.enum
  }
  if ('minItems' in value && value.minItems !== undefined) {
    if (typeof value.minItems !== 'number' || !Number.isFinite(value.minItems)) return null
    field.minItems = value.minItems
  }
  if ('maxItems' in value && value.maxItems !== undefined) {
    if (typeof value.maxItems !== 'number' || !Number.isFinite(value.maxItems)) return null
    field.maxItems = value.maxItems
  }
  if ('itemPattern' in value && value.itemPattern !== undefined) {
    if (typeof value.itemPattern !== 'string') return null
    field.itemPattern = value.itemPattern
  }
  if ('joinedSeparator' in value && value.joinedSeparator !== undefined) {
    if (typeof value.joinedSeparator !== 'string') return null
    field.joinedSeparator = value.joinedSeparator
  }
  if ('joinedMaxLength' in value && value.joinedMaxLength !== undefined) {
    if (typeof value.joinedMaxLength !== 'number' || !Number.isFinite(value.joinedMaxLength))
      return null
    field.joinedMaxLength = value.joinedMaxLength
  }
  return field
}

function parseSourceAddon(value: unknown): SourceAddon | null {
  if (value === null || typeof value !== 'object') return null
  if (!('uuid' in value) || typeof value.uuid !== 'string' || value.uuid === '') return null
  if (!('name' in value) || typeof value.name !== 'string' || value.name === '') return null
  if (!('alias' in value) || typeof value.alias !== 'string') return null
  return {
    uuid: value.uuid,
    name: value.name,
    alias: value.alias === '' ? value.name : value.alias,
  }
}

function parseSourceSubmission(value: unknown): SourceSubmission | null {
  if (value === null || typeof value !== 'object') return null
  if (!('id' in value) || typeof value.id !== 'number' || !Number.isFinite(value.id)) return null
  if (!('kind' in value) || (value.kind !== 'new' && value.kind !== 'update')) return null
  if (!('name' in value) || typeof value.name !== 'string' || value.name === '') return null
  const status =
    'status' in value && typeof value.status === 'string' && value.status !== ''
      ? value.status
      : undefined
  const createdAt =
    'created_at' in value && typeof value.created_at === 'string' && value.created_at !== ''
      ? value.created_at
      : undefined
  return { id: value.id, kind: value.kind, name: value.name, status, createdAt }
}

function mapWireValue(field: SchemaField, raw: unknown): WireValue | 'error' {
  const list = field.widget === 'enum-multi' || field.widget === 'string-list'
  if (list) {
    if (raw === undefined || raw === null) return []
    if (!Array.isArray(raw) || !raw.every(item => typeof item === 'string')) return 'error'
    return raw
  }
  if (raw === undefined || raw === null) return ''
  if (typeof raw !== 'string') return 'error'
  return raw
}

function parseErrorItems(data: object): { field: string; message: string }[] {
  if (!('errors' in data) || !Array.isArray(data.errors)) return []
  const errors: { field: string; message: string }[] = []
  for (const error of data.errors) {
    if (error === null || typeof error !== 'object') continue
    if (!('field' in error) || !('message' in error)) continue
    if (typeof error.field !== 'string' || typeof error.message !== 'string') continue
    errors.push({ field: error.field, message: error.message })
  }
  return errors
}

function dataId(envelope: Envelope | null): number | null {
  if (envelope === null || envelope.data === null || typeof envelope.data !== 'object') return null
  if (
    !('id' in envelope.data) ||
    typeof envelope.data.id !== 'number' ||
    !Number.isFinite(envelope.data.id)
  ) {
    return null
  }
  return envelope.data.id
}

function closedConflict(
  statusCode: number,
  envelope: Envelope | null
): { status: 'not_open'; id: number } | null {
  if (statusCode !== 409 || envelope === null || envelope.message !== 'submission not open')
    return null
  const id = dataId(envelope)
  if (id === null) return null
  return { status: 'not_open', id }
}

function isNotFound(statusCode: number, envelope: Envelope | null): boolean {
  if (statusCode === 400 && envelope?.message === 'invalid submission id') return true
  return statusCode === 404 && envelope?.message === 'not found'
}
