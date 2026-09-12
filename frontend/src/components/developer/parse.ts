import type {
  AddonSchema,
  AddonSources,
  DeclarationKind,
  DeclarationValues,
  FieldErrors,
  SchemaField,
  SourceAddon,
  SourceSubmission,
  SubmissionDetail,
  SubmissionMessage,
  SubmissionStatus,
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

export type ParseWithdrawResult =
  | { status: 'withdrawn'; id: number; revision: number }
  | { status: 'not_open'; id: number }
  | { status: 'not_found'; message: string }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

export type ParseSubmissionDetailResult =
  | { status: 'ok'; submission: SubmissionDetail }
  | { status: 'not_found'; message: string }
  | { status: 'unauthorized'; message: string }
  | { status: 'error'; message: string }

const SUBMISSION_STATUSES: Record<string, true> = {
  open: true,
  approved: true,
  rejected: true,
}

const WIDGETS: Record<string, true> = {
  text: true,
  textarea: true,
  'enum-multi': true,
  'string-list': true,
}
const STRING_FIELD_OPTIONS = ['hint', 'pattern', 'itemPattern', 'joinedSeparator'] as const
const NUMBER_FIELD_OPTIONS = ['maxLength', 'minItems', 'maxItems', 'joinedMaxLength'] as const

const UNSUPPORTED_SCHEMA = 'This app needs an update to edit addon declarations.'
const SOURCE_UNAVAILABLE = 'This source is unavailable.'

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object'
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(item => typeof item === 'string')
}

function nonEmptyString(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null
}

export function parseEnvelope(body: unknown): Envelope | null {
  if (!isObject(body) || !('data' in body)) return null
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
  if (statusCode !== 200 || !envelope?.status) {
    return { status: 'error', message: envelope?.message || 'Unexpected schema response.' }
  }
  const data = envelope.data
  if (!isObject(data) || typeof data.version !== 'number') {
    return { status: 'error', message: 'Unexpected schema response.' }
  }
  if (data.version !== 1) {
    return { status: 'unsupported', message: UNSUPPORTED_SCHEMA }
  }
  if (!Array.isArray(data.widgets) || !Array.isArray(data.fields)) {
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
  if (statusCode !== 200 || !envelope?.status) {
    return { status: 'error', message: envelope?.message || 'Unexpected sources response.' }
  }
  const data = envelope.data
  if (!isObject(data) || !Array.isArray(data.addons) || !Array.isArray(data.submissions)) {
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
  if (statusCode !== 200 || !envelope?.status) {
    return { status: 'error', message: envelope?.message || 'Unexpected values response.' }
  }
  const data = envelope.data
  if (!isObject(data) || (data.kind !== 'new' && data.kind !== 'update')) {
    return { status: 'error', message: 'Unexpected values response.' }
  }
  if (!isObject(data.values) || Array.isArray(data.values)) {
    return { status: 'error', message: 'Unexpected values response.' }
  }
  const values: DeclarationValues = {}
  for (const field of schema.fields) {
    const mapped = mapWireValue(field, data.values[field.key])
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
  if (statusCode !== 200 || !envelope?.status) {
    return { status: 'error', message: envelope?.message || 'Unexpected validation response.' }
  }
  const data = envelope.data
  if (!isObject(data) || typeof data.valid !== 'boolean') {
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
    !envelope.status &&
    envelope.message === 'invalid addon'
  ) {
    const data = envelope.data
    if (isObject(data) && data.valid === false && Array.isArray(data.errors)) {
      return { status: 'invalid', ...groupValidationErrors(parseErrorItems(data), schemaKeys) }
    }
  }
  if (statusCode !== 200 || !envelope?.status) {
    return { status: 'error', message: envelope?.message || 'Unexpected save response.' }
  }
  const data = envelope.data
  if (!isObject(data) || !isFiniteNumber(data.id) || !isFiniteNumber(data.revision)) {
    return { status: 'error', message: 'Unexpected save response.' }
  }
  return { status: 'saved', id: data.id, revision: data.revision }
}

export function parseWithdrawResponse(statusCode: number, body: unknown): ParseWithdrawResult {
  const envelope = parseEnvelope(body)
  if (statusCode === 401) {
    return { status: 'unauthorized', message: 'Sign in to withdraw a submission.' }
  }
  const closed = closedConflict(statusCode, envelope)
  if (closed) return closed
  if (isNotFound(statusCode, envelope)) {
    return { status: 'not_found', message: SOURCE_UNAVAILABLE }
  }
  if (statusCode !== 200 || !envelope?.status) {
    return { status: 'error', message: envelope?.message || 'Unexpected withdraw response.' }
  }
  const data = envelope.data
  if (
    !isObject(data) ||
    !isFiniteNumber(data.id) ||
    !isFiniteNumber(data.revision) ||
    data.status !== 'withdrawn'
  ) {
    return { status: 'error', message: 'Unexpected withdraw response.' }
  }
  return { status: 'withdrawn', id: data.id, revision: data.revision }
}

export function parseSubmissionDetail(
  statusCode: number,
  body: unknown
): ParseSubmissionDetailResult {
  const envelope = parseEnvelope(body)
  if (statusCode === 401) {
    return { status: 'unauthorized', message: 'Sign in to view this submission.' }
  }
  if (isNotFound(statusCode, envelope)) {
    return { status: 'not_found', message: 'This submission is unavailable.' }
  }
  if (statusCode !== 200 || !envelope?.status) {
    return { status: 'error', message: envelope?.message || 'Unexpected submission response.' }
  }
  const data = envelope.data
  if (
    !isObject(data) ||
    !isFiniteNumber(data.id) ||
    (data.kind !== 'new' && data.kind !== 'update') ||
    typeof data.status !== 'string' ||
    !SUBMISSION_STATUSES[data.status]
  ) {
    return { status: 'error', message: 'Unexpected submission response.' }
  }
  return {
    status: 'ok',
    submission: {
      id: data.id,
      kind: data.kind,
      status: data.status as SubmissionStatus,
      targetAddonUuid: nonEmptyString(data.target_addon_uuid),
      revision: isFiniteNumber(data.revision) ? data.revision : 0,
      createdAt: nonEmptyString(data.created_at),
      approvedAt: nonEmptyString(data.approved_at),
      rejectedAt: nonEmptyString(data.rejected_at),
      messages: parseSubmissionMessages(data),
    },
  }
}

function parseSubmissionMessages(value: Record<string, unknown>): SubmissionMessage[] {
  if (!Array.isArray(value.messages)) return []
  const messages: SubmissionMessage[] = []
  for (const item of value.messages) {
    if (!isObject(item) || !isFiniteNumber(item.id) || typeof item.body !== 'string') continue
    messages.push({
      id: item.id,
      body: item.body,
      revision: isFiniteNumber(item.revision) ? item.revision : 0,
      createdAt: nonEmptyString(item.created_at),
    })
  }
  return messages
}

function isWidget(value: unknown): value is Widget {
  return typeof value === 'string' && WIDGETS[value]
}

function parseSchemaField(value: unknown): SchemaField | 'unsupported' | null {
  if (!isObject(value)) return null
  if (typeof value.key !== 'string' || value.key === '') return null
  if (typeof value.label !== 'string' || typeof value.section !== 'string') return null
  if (typeof value.widget !== 'string') return null
  if (!isWidget(value.widget)) return 'unsupported'
  if (typeof value.required !== 'boolean') return null
  const field: SchemaField = {
    key: value.key,
    label: value.label,
    section: value.section,
    widget: value.widget,
    required: value.required,
  }
  for (const key of STRING_FIELD_OPTIONS) {
    const option = value[key]
    if (option === undefined) continue
    if (typeof option !== 'string') return null
    field[key] = option
  }
  for (const key of NUMBER_FIELD_OPTIONS) {
    const option = value[key]
    if (option === undefined) continue
    if (!isFiniteNumber(option)) return null
    field[key] = option
  }
  if (value.immutable !== undefined) {
    if (typeof value.immutable !== 'boolean') return null
    field.immutable = value.immutable
  }
  if (value.lengthUnit !== undefined) {
    if (value.lengthUnit !== 'runes' && value.lengthUnit !== 'bytes') return null
    field.lengthUnit = value.lengthUnit
  }
  if (value.enum !== undefined) {
    if (!isStringArray(value.enum)) return null
    field.enum = value.enum
  }
  return field
}

function parseSourceAddon(value: unknown): SourceAddon | null {
  if (!isObject(value)) return null
  if (typeof value.uuid !== 'string' || value.uuid === '') return null
  if (typeof value.name !== 'string' || value.name === '') return null
  if (typeof value.alias !== 'string') return null
  return {
    uuid: value.uuid,
    name: value.name,
    alias: value.alias === '' ? value.name : value.alias,
  }
}

function parseSourceSubmission(value: unknown): SourceSubmission | null {
  if (!isObject(value) || !isFiniteNumber(value.id)) return null
  if (value.kind !== 'new' && value.kind !== 'update') return null
  if (typeof value.name !== 'string' || value.name === '') return null
  const status = nonEmptyString(value.status) ?? undefined
  const createdAt = nonEmptyString(value.created_at) ?? undefined
  return { id: value.id, kind: value.kind, name: value.name, status, createdAt }
}

function mapWireValue(field: SchemaField, raw: unknown): WireValue | 'error' {
  const list = field.widget === 'enum-multi' || field.widget === 'string-list'
  if (list) {
    if (raw === undefined || raw === null) return []
    if (!isStringArray(raw)) return 'error'
    return raw
  }
  if (raw === undefined || raw === null) return ''
  if (typeof raw !== 'string') return 'error'
  return raw
}

function parseErrorItems(data: Record<string, unknown>): { field: string; message: string }[] {
  if (!Array.isArray(data.errors)) return []
  const errors: { field: string; message: string }[] = []
  for (const error of data.errors) {
    if (!isObject(error)) continue
    if (typeof error.field !== 'string' || typeof error.message !== 'string') continue
    errors.push({ field: error.field, message: error.message })
  }
  return errors
}

function dataId(envelope: Envelope | null): number | null {
  const data = envelope?.data
  return isObject(data) && isFiniteNumber(data.id) ? data.id : null
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
