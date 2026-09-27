import { CheckIcon } from 'lucide-react'
import { type RefObject, useEffect, useState } from 'react'

import type {
  SetDeclarationField,
  SetDeclarationValue,
} from '@/components/developer/AddonDeclarationFields'
import { isPublishFormDirty, type PublishFormState } from '@/components/developer/constants'
import {
  FORM_FIELD_ORDER,
  scrollFirstFieldErrorIntoView,
} from '@/components/developer/declarationFields'
import { valuesToForm } from '@/components/developer/formValues.ts'
import {
  getAddonSchema,
  requireAddonSchema,
  schemaZeroValues,
} from '@/components/developer/schema.ts'
import type { AddonSchema, DeclarationKind, EditorSource } from '@/components/developer/types.ts'
import { type FieldErrors, submitAddon, validateAddon } from '@/components/developer/validate'
import { getAddonValues } from '@/components/developer/values.ts'
import { toast } from '@/components/ui/toast'

export interface DeclarationFormOptions {
  initial: PublishFormState
  /** Submission being revised, null creates a new submission. */
  initialSubmissionId?: number | null
  /** Where to load an existing submission from when the user chooses to resume it. */
  resumeSource: (id: number) => EditorSource
  /** Called after an existing submission has been loaded into the form. */
  onResumed?: (kind: DeclarationKind) => void
  /** Called after the submission is created or updated. */
  onSubmitted: () => void
  /** Scroll area holding the fields, used to bring the first error into view. */
  scrollContainer?: RefObject<HTMLElement | null>
}

export function useDeclarationForm({
  initial,
  initialSubmissionId = null,
  resumeSource,
  onResumed,
  onSubmitted,
  scrollContainer,
}: DeclarationFormOptions) {
  const [form, setForm] = useState<PublishFormState>(initial)
  const [submissionId, setSubmissionId] = useState<number | null>(initialSubmissionId)
  const [alreadyOpenId, setAlreadyOpenId] = useState<number | null>(null)
  const [validating, setValidating] = useState(false)
  const [validated, setValidated] = useState(false)
  const [submitting, setSubmitting] = useState(false)
  const [resuming, setResuming] = useState(false)
  const [editable, setEditable] = useState(true)
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})
  const [validationError, setValidationError] = useState(false)
  const [publishError, setPublishError] = useState<string | null>(null)

  const [schema, setSchema] = useState<AddonSchema | null>(null)
  const [schemaError, setSchemaError] = useState<string | null>(null)
  useEffect(() => {
    let cancelled = false
    void getAddonSchema().then(result => {
      if (cancelled) return
      if (result.status !== 'ok') {
        setSchemaError(result.message)
        return
      }
      setSchema(result.schema)
      setForm(prev => ({
        ...prev,
        values: { ...schemaZeroValues(result.schema), ...prev.values },
      }))
    })
    return () => {
      cancelled = true
    }
  }, [])

  const busy = validating || submitting || resuming || !editable
  const dirty = submissionId !== initialSubmissionId || isPublishFormDirty(form, initial)
  const hasFieldErrors = FORM_FIELD_ORDER.some(key => !!fieldErrors[key]?.length)
  const failCopy = hasFieldErrors
    ? 'Fix the highlighted fields.'
    : publishError !== null
      ? publishError
      : validationError
        ? "Couldn't validate this addon."
        : null

  const clearResult = () => {
    setValidated(false)
    setValidationError(false)
    setPublishError(null)
  }

  const clearFieldError = (key: string) => {
    setFieldErrors(prev => {
      if (!prev[key]) return prev
      const next = { ...prev }
      delete next[key]
      return next
    })
  }

  const setField: SetDeclarationField = (key, value) => {
    setForm(prev => {
      const nextValue =
        typeof value === 'function'
          ? (value as (current: PublishFormState[typeof key]) => PublishFormState[typeof key])(
              prev[key]
            )
          : value
      if (Object.is(nextValue, prev[key])) return prev
      return { ...prev, [key]: nextValue }
    })
    clearResult()
    clearFieldError('icon')
  }

  const setValue: SetDeclarationValue = (key, value) => {
    setForm(prev => {
      const nextValue = typeof value === 'function' ? value(prev.values[key]) : value
      if (Object.is(nextValue, prev.values[key])) return prev
      return { ...prev, values: { ...prev.values, [key]: nextValue } }
    })
    clearResult()
    clearFieldError(key)
  }

  const applyInvalidResult = (fields: FieldErrors) => {
    const hasFields = FORM_FIELD_ORDER.some(key => !!fields[key]?.length)
    if (hasFields) {
      setFieldErrors(fields)
      requestAnimationFrame(() => {
        scrollFirstFieldErrorIntoView(fields, scrollContainer?.current)
      })
      return
    }
    setValidationError(true)
  }

  const markNotOpen = () => {
    setEditable(false)
    setSubmissionId(null)
    setPublishError('This submission is no longer open.')
  }

  const resume = async (id: number) => {
    if (busy) return
    setResuming(true)
    try {
      const schema = await requireAddonSchema()
      if (!schema) {
        setPublishError('This source is unavailable.')
        return
      }
      const result = await getAddonValues(resumeSource(id), schema)
      if (result.status !== 'ok') {
        setPublishError(result.message)
        return
      }
      setSubmissionId(id)
      setForm(valuesToForm(result.values))
      clearResult()
      setFieldErrors({})
      setEditable(true)
      onResumed?.(result.kind)
    } finally {
      setResuming(false)
    }
  }

  const validate = async () => {
    if (busy) return
    setValidating(true)
    setPublishError(null)
    setFieldErrors({})
    setValidationError(false)
    let validationPassed = false
    try {
      const result = await validateAddon(form, submissionId)
      if (result.status === 'not_open') {
        markNotOpen()
        return
      }
      if (result.status === 'already_open') {
        setAlreadyOpenId(result.id)
        return
      }
      if (result.status === 'error') {
        setValidationError(true)
        return
      }
      if (result.status === 'valid') {
        validationPassed = true
        return
      }
      applyInvalidResult(result.fields)
    } catch {
      setValidationError(true)
    } finally {
      setValidated(validationPassed)
      setValidating(false)
    }
  }

  const submit = async () => {
    if (busy || !validated) return
    setSubmitting(true)
    setPublishError(null)
    setFieldErrors({})
    try {
      const result = await submitAddon(form, submissionId)
      if (result.status === 'submitted') {
        const created = submissionId === null
        toast({
          title: created ? 'Addon submitted' : 'Submission updated',
          description: created ? "It's now in review." : 'Your changes were saved.',
          icon: CheckIcon,
        })
        onSubmitted()
        return
      }
      if (result.status === 'already_open') {
        setAlreadyOpenId(result.id)
        return
      }
      if (result.status === 'invalid') {
        setValidated(false)
        applyInvalidResult(result.fields)
        return
      }
      if (result.status === 'not_open') {
        markNotOpen()
        return
      }
      setPublishError(result.message)
    } catch {
      setPublishError("Couldn't publish this addon.")
    } finally {
      setSubmitting(false)
    }
  }

  /** Validates first, then submits once the declaration has passed validation. */
  const primaryAction = () => {
    if (validated) {
      void submit()
      return
    }
    void validate()
  }

  const resumeAlreadyOpen = () => {
    const id = alreadyOpenId
    setAlreadyOpenId(null)
    if (id === null) return
    void resume(id)
  }

  return {
    form,
    setField,
    setValue,
    schema,
    schemaError,
    submissionId,
    fieldErrors,
    publishError,
    failCopy,
    validating,
    validated,
    submitting,
    busy,
    dirty,
    primaryAction,
    alreadyOpen: alreadyOpenId !== null,
    dismissAlreadyOpen: () => setAlreadyOpenId(null),
    resumeAlreadyOpen,
  }
}
