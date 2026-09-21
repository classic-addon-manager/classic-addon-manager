import type { PublishFormState } from '@/components/developer/constants'
import type { DeclarationValues } from '@/components/developer/types.ts'

export function formToValues(form: PublishFormState): DeclarationValues {
  return {
    ...form.values,
    // Absent preserves the server's current icon state, '' removes, a UUID replaces.
    ...(form.iconAssetId === null ? {} : { icon_asset_id: form.iconAssetId }),
  }
}

export function valuesToForm(values: DeclarationValues): PublishFormState {
  const { icon_url, ...rest } = values
  return {
    values: rest,
    iconAssetId: null,
    iconUrl: typeof icon_url === 'string' && icon_url !== '' ? icon_url : null,
  }
}

export function textOf(values: DeclarationValues, key: string): string {
  const value = values[key]
  return typeof value === 'string' ? value : ''
}

export function listOf(values: DeclarationValues, key: string): string[] {
  const value = values[key]
  return Array.isArray(value) ? value : []
}
