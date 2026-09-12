export type Widget = 'text' | 'textarea' | 'enum-multi' | 'string-list'
export type WireValue = string | string[]
export type DeclarationValues = Record<string, WireValue>
export type FieldErrors = Record<string, string[]>
export type DeclarationKind = 'new' | 'update'

export type SchemaField = {
  key: string
  label: string
  section: string
  widget: Widget
  required: boolean
  hint?: string
  immutable?: boolean
  pattern?: string
  maxLength?: number
  lengthUnit?: 'runes' | 'bytes'
  enum?: string[]
  minItems?: number
  maxItems?: number
  itemPattern?: string
  joinedSeparator?: string
  joinedMaxLength?: number
}

export type AddonSchema = { version: number; widgets: Widget[]; fields: SchemaField[] }

export type SourceAddon = { uuid: string; name: string; alias: string }
export type SourceSubmission = {
  id: number
  kind: DeclarationKind
  name: string
  status?: string
  createdAt?: string
}
export type AddonSources = { addons: SourceAddon[]; submissions: SourceSubmission[] }

export type SubmissionStatus = 'open' | 'approved' | 'rejected' | 'withdrawn'
export type SubmissionMessage = {
  id: number
  body: string
  revision: number
  createdAt: string | null
}

export type SubmissionDetail = {
  id: number
  kind: DeclarationKind
  status: SubmissionStatus
  targetAddonUuid: string | null
  revision: number
  createdAt: string | null
  approvedAt: string | null
  rejectedAt: string | null
  messages: SubmissionMessage[]
}

export type EditorSource =
  | { type: 'new' }
  | { type: 'addon'; uuid: string; name: string; alias: string }
  | { type: 'submission'; id: number; kind: DeclarationKind; name: string }

export const V1_FIELD_KEYS = [
  'name',
  'alias',
  'description',
  'author',
  'repo',
  'branch',
  'tags',
  'keywords',
  'dependencies',
  'kofi',
] as const

export function v1SchemaKeys(): Set<string> {
  return new Set(V1_FIELD_KEYS)
}
