import type { DeclarationValues } from '@/components/developer/types.ts'

export interface PublishFormState {
  /** Wire-typed values keyed by schema field key. */
  values: DeclarationValues
  /** Opaque temporary asset ID from upload; null means preserve current icon. */
  iconAssetId: string | null
  /** Server-projected preview URL; display only, never submitted or reconstructed. */
  iconUrl: string | null
}

export const INITIAL_PUBLISH_FORM: PublishFormState = {
  values: {},
  iconAssetId: null,
  iconUrl: null,
}

export function isPublishFormDirty(
  form: PublishFormState,
  baseline: PublishFormState = INITIAL_PUBLISH_FORM
): boolean {
  const keys = new Set([...Object.keys(form.values), ...Object.keys(baseline.values)])
  for (const key of keys) {
    const a = form.values[key]
    const b = baseline.values[key]
    if (Array.isArray(a) || Array.isArray(b)) {
      const listA = Array.isArray(a) ? a : []
      const listB = Array.isArray(b) ? b : []
      if (listA.length !== listB.length || listA.some((item, i) => item !== listB[i])) return true
      continue
    }
    if (
      (a === undefined && (b === '' || b === false)) ||
      (b === undefined && (a === '' || a === false))
    ) {
      continue
    }
    if (a !== b) return true
  }
  return form.iconAssetId !== baseline.iconAssetId
}

export function publishFormFromPayload(payload: DeclarationValues): PublishFormState {
  return {
    values: { ...payload },
    iconAssetId: null,
    iconUrl: null,
  }
}
