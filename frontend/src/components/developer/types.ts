export type Widget = 'text' | 'textarea' | 'enum-multi' | 'string-list' | 'checkbox'
export type WireValue = string | string[] | boolean
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
  half?: boolean
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

export type SourceAddon = {
  uuid: string
  name: string
  alias: string
  downloads: number
}
export type SourceSubmission = {
  id: number
  kind: DeclarationKind
  name: string
  status?: string
  createdAt?: string
}
export type AddonSources = { addons: SourceAddon[]; submissions: SourceSubmission[] }

export type SubmissionStatus = 'open' | 'approved' | 'rejected'
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

export type VersionLagBucket = 'latest' | 'one_behind' | 'two_behind' | 'unavailable'

export type SnapshotPoint = {
  takenOn: string
  downloads: number
  subscribers: number
  dailyDownloads: number | null
  dailySubscriberChange: number | null
}

export type AddonDownloadStats = {
  total: number
  series: SnapshotPoint[]
}

export type AddonRatingStats = {
  likes: number
  dislikes: number
  totalVotes: number
  likePercentage: number | null
  deletedAccounts: number
}

export type AddonSubscriberStats = {
  current: number
  series: SnapshotPoint[]
}

export type AddonVersionUsage = {
  releaseId: number
  tagName: string
  subscribers: number
  share: number
  lagBucket: VersionLagBucket
}

export type AddonVersionStats = {
  buckets: Record<VersionLagBucket, number>
  share: AddonVersionUsage[]
}

export type AddonDeveloperStats = {
  downloads: AddonDownloadStats
  ratings: AddonRatingStats
  subscribers: AddonSubscriberStats
  versions: AddonVersionStats
}

export const SCHEMA_FIELD_KEYS = [
  'name',
  'alias',
  'description',
  'author',
  'repo',
  'branch',
  'tags',
  'keywords',
  'dependencies',
  'library',
  'kofi',
] as const
