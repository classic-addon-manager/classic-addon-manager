import type { PublishFormState } from '@/components/developer/constants'
import type { FieldErrors } from '@/components/developer/validate'

export const FORM_FIELD_ORDER: (keyof PublishFormState)[] = [
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
]

export function scrollFirstFieldErrorIntoView(
  errors: FieldErrors,
  container: HTMLElement | null = null
) {
  const firstKey = FORM_FIELD_ORDER.find(key => !!errors[key]?.length)
  if (!firstKey) return
  const target = document.getElementById(`addon-${firstKey}`)
  if (!target) return

  const viewport = target.closest('[data-slot="scroll-area-viewport"]')
  const scroller =
    container !== null && container.scrollHeight > container.clientHeight
      ? container
      : viewport instanceof HTMLElement
        ? viewport
        : container
  if (scroller) {
    const mainRect = scroller.getBoundingClientRect()
    const targetRect = target.getBoundingClientRect()
    const fullyVisible = targetRect.top >= mainRect.top && targetRect.bottom <= mainRect.bottom
    if (fullyVisible) return
  }

  target.scrollIntoView({ behavior: 'smooth', block: 'center' })
}
