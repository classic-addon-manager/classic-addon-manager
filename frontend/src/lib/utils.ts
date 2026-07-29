import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Extract a human-readable message from Wails/runtime errors and other throwables */
export function getErrorMessage(error: unknown, fallback = 'An unknown error occurred'): string {
  const message = extractErrorMessage(error)
  if (!message) return fallback
  return sentenceCase(message)
}

function sentenceCase(message: string): string {
  const first = message.charAt(0)
  if (!first || first === first.toUpperCase()) return message
  return first.toUpperCase() + message.slice(1)
}

function extractErrorMessage(error: unknown, depth = 0): string | null {
  if (error == null || depth > 6) return null

  if (typeof error === 'string') {
    const trimmed = error.trim()
    if (!trimmed) return null

    const parsed = tryParseJsonObject(trimmed)
    if (parsed) {
      return extractErrorMessage(parsed, depth + 1) ?? trimmed
    }

    return trimmed
  }

  if (error instanceof Error) {
    // Prefer a structured message over the raw Error.message when Wails embeds JSON
    const fromMessage = extractErrorMessage(error.message, depth + 1)
    if (fromMessage) return fromMessage

    const cause = (error as Error & { cause?: unknown }).cause
    const fromCause = extractErrorMessage(cause, depth + 1)
    if (fromCause) return fromCause

    return null
  }

  if (typeof error === 'object') {
    const record = error as Record<string, unknown>

    if ('message' in record) {
      const fromMessage = extractErrorMessage(record.message, depth + 1)
      if (fromMessage) return fromMessage
    }

    if ('error' in record) {
      const fromError = extractErrorMessage(record.error, depth + 1)
      if (fromError) return fromError
    }

    if ('cause' in record) {
      const cause = record.cause
      // Wails may send an empty cause object — ignore those.
      if (cause != null && !isEmptyObject(cause)) {
        const fromCause = extractErrorMessage(cause, depth + 1)
        if (fromCause) return fromCause
      }
    }
  }

  return null
}

function tryParseJsonObject(value: string): Record<string, unknown> | null {
  if (!value.startsWith('{')) return null
  try {
    const parsed: unknown = JSON.parse(value)
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>
    }
  } catch {
    // Not JSON, treat as a plain string message.
  }
  return null
}

function isEmptyObject(value: unknown): boolean {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    Object.keys(value as object).length === 0
  )
}

export const safeCall = async <T>(promise: Promise<T>): Promise<[T | null, Error | null]> => {
  try {
    const result = await promise
    return [result, null]
  } catch (error) {
    return [null, new Error(getErrorMessage(error))]
  }
}

/**
 * Formats a go date string to a human-readable date string.
 */
export function formatToLocalTime(
  dateString: string,
  monthFormat: 'short' | 'long' = 'long'
): string {
  const date = new Date(dateString)
  const options: Intl.DateTimeFormatOptions = {
    hour: '2-digit',
    minute: '2-digit',
    day: '2-digit',
    month: monthFormat,
    year: 'numeric',
  }
  return date.toLocaleString(undefined, options).replace(',', '')
}

/**
 * Takes a go date and determines how long ago it was in days.
 */
export function daysAgo(dateString: string): number {
  const date = new Date(dateString)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  return Math.floor(diff / (1000 * 60 * 60 * 24))
}
