export type AccentColorId =
  | 'gold'
  | 'amber'
  | 'rose'
  | 'violet'
  | 'blue'
  | 'cyan'
  | 'emerald'
  | 'slate'

export interface AccentPreset {
  id: AccentColorId
  label: string
  l: number
  c: number
  h: number
}

// Lightness stays at or above 0.70 so the near-black --primary-foreground
// remains legible on top of every preset.
export const ACCENT_PRESETS: readonly AccentPreset[] = [
  { id: 'gold', label: 'Gold', l: 0.83, c: 0.095, h: 80 },
  { id: 'amber', label: 'Amber', l: 0.8, c: 0.14, h: 65 },
  { id: 'rose', label: 'Rose', l: 0.72, c: 0.16, h: 15 },
  { id: 'violet', label: 'Violet', l: 0.7, c: 0.17, h: 295 },
  { id: 'blue', label: 'Blue', l: 0.7, c: 0.14, h: 250 },
  { id: 'cyan', label: 'Cyan', l: 0.78, c: 0.11, h: 195 },
  { id: 'emerald', label: 'Emerald', l: 0.78, c: 0.14, h: 155 },
  { id: 'slate', label: 'Slate', l: 0.8, c: 0.02, h: 260 },
]

export const DEFAULT_ACCENT_COLOR: AccentColorId = 'gold'

const PRIMARY_FOREGROUND = 'oklch(0.205 0 0)'

const CHART_STEPS = 5
const CHART_LIGHTNESS_STEP = 0.08
const CHART_CHROMA_STEP = 0.005

const round = (value: number) => Math.round(value * 1000) / 1000

const oklch = (l: number, c: number, h: number) => `oklch(${round(l)} ${round(c)} ${h})`

export const accentSwatch = (preset: AccentPreset) => oklch(preset.l, preset.c, preset.h)

export const getAccentPreset = (id: AccentColorId) =>
  ACCENT_PRESETS.find(preset => preset.id === id) ??
  ACCENT_PRESETS.find(preset => preset.id === DEFAULT_ACCENT_COLOR)!

export const applyAccentColor = (id: AccentColorId) => {
  if (typeof document === 'undefined') return

  const preset = getAccentPreset(id)
  const accent = accentSwatch(preset)
  const { style } = document.documentElement

  style.setProperty('--primary', accent)
  style.setProperty('--primary-foreground', PRIMARY_FOREGROUND)
  style.setProperty('--ring', accent)
  style.setProperty('--sidebar-primary', accent)
  style.setProperty('--sidebar-ring', accent)

  for (let step = 0; step < CHART_STEPS; step++) {
    style.setProperty(
      `--chart-${step + 1}`,
      oklch(preset.l - step * CHART_LIGHTNESS_STEP, preset.c + step * CHART_CHROMA_STEP, preset.h)
    )
  }
}
