import { Check } from 'lucide-react'

import { ACCENT_PRESETS, accentSwatch } from '@/lib/accentColors'
import { cn } from '@/lib/utils'
import { usePreferencesStore } from '@/stores/preferencesStore'

export const AccentColorPicker = () => {
  const accentColor = usePreferencesStore(state => state.accentColor)
  const setAccentColor = usePreferencesStore(state => state.setAccentColor)

  return (
    <div className="flex flex-wrap items-center gap-3 px-4 py-3.5">
      {ACCENT_PRESETS.map(preset => {
        const swatch = accentSwatch(preset)
        const isSelected = preset.id === accentColor

        return (
          <button
            key={preset.id}
            type="button"
            onClick={() => setAccentColor(preset.id)}
            aria-pressed={isSelected}
            aria-label={preset.label}
            title={preset.label}
            className={cn(
              'flex h-8 w-8 items-center justify-center rounded-full transition-transform',
              'focus-visible:outline-none hover:scale-110',
              isSelected && 'scale-110'
            )}
            style={{
              backgroundColor: swatch,
              boxShadow: isSelected
                ? `0 0 0 2px var(--card), 0 0 0 4px ${swatch}`
                : '0 0 0 1px rgba(0, 0, 0, 0.25)',
            }}
          >
            {isSelected && <Check className="h-4 w-4 text-primary-foreground" strokeWidth={3} />}
          </button>
        )
      })}
    </div>
  )
}
