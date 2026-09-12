import { Check, LoaderCircle } from 'lucide-react'
import { useState } from 'react'

import {
  AddonDeclarationFields,
  type SetDeclarationField,
} from '@/components/developer/AddonDeclarationFields'
import {
  isPublishFormDirty,
  publishFormFromPayload,
  type PublishFormState,
} from '@/components/developer/constants'
import {
  FORM_FIELD_ORDER,
  scrollFirstFieldErrorIntoView,
} from '@/components/developer/declarationFields'
import { valuesToForm } from '@/components/developer/formValues.ts'
import type { OwnedAddon } from '@/components/developer/ownedParse'
import { requireAddonSchema } from '@/components/developer/schema.ts'
import { type FieldErrors, submitAddon, validateAddon } from '@/components/developer/validate'
import { getAddonValues } from '@/components/developer/values.ts'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast'
import { cn } from '@/lib/utils'

export function AddonEditPanel({
  addon,
  initialForm,
  submitLabel,
  onCancel,
  onSubmitted,
  initialSubmissionId,
  nameLocked = true,
  requireChanges = true,
  introTitle = 'Edit listing details',
  introNote = 'Leaving this form discards unsent edits. Your published listing stays unchanged until approval.',
}: {
  addon: OwnedAddon
  initialForm: PublishFormState | null
  submitLabel: string
  onCancel: () => void
  /** Invoked after a submission is created or updated. */
  onSubmitted?: () => void
  /** Pinned submission being revised, null creates a new submission. */
  initialSubmissionId?: number | null
  nameLocked?: boolean
  /** Require unsaved changes before validating, off for resubmitting an unchanged declaration. */
  requireChanges?: boolean
  introTitle?: string
  introNote?: string
}) {
  const initial =
    initialForm ??
    publishFormFromPayload({
      name: addon.name,
      alias: addon.alias,
      description: addon.description,
      author: addon.author,
      repo: addon.repo,
      branch: addon.branch ?? '',
      tags: addon.tags,
      keywords: [],
      dependencies: [],
      kofi: '',
    })
  const [form, setForm] = useState<PublishFormState>(initial)
  const [submissionId, setSubmissionId] = useState<number | null>(initialSubmissionId ?? null)
  const [alreadyOpenId, setAlreadyOpenId] = useState<number | null>(null)
  const [validating, setValidating] = useState(false)
  const [validated, setValidated] = useState(false)
  const [saving, setSaving] = useState(false)
  const [resuming, setResuming] = useState(false)
  const [editable, setEditable] = useState(true)
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})
  const [validationError, setValidationError] = useState(false)
  const [publishError, setPublishError] = useState<string | null>(null)
  const busy = validating || saving || resuming || !editable
  const dirty = submissionId !== (initialSubmissionId ?? null) || isPublishFormDirty(form, initial)
  const ctaDisabled = busy || (!validated && requireChanges && !dirty)
  const hasFieldErrors = FORM_FIELD_ORDER.some(key => !!fieldErrors[key]?.length)
  const failCopy = hasFieldErrors
    ? 'Fix the highlighted fields.'
    : publishError !== null
      ? publishError
      : validationError
        ? "Couldn't validate this addon."
        : null
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
    setValidated(false)
    setValidationError(false)
    setPublishError(null)
    setFieldErrors(prev => {
      if (!prev[key]) return prev
      const next = { ...prev }
      delete next[key]
      return next
    })
  }

  const applyInvalidResult = (fields: FieldErrors) => {
    const hasFields = FORM_FIELD_ORDER.some(key => !!fields[key]?.length)
    if (hasFields) {
      setFieldErrors(fields)
      requestAnimationFrame(() => {
        scrollFirstFieldErrorIntoView(fields)
      })
      return
    }
    setValidationError(true)
  }

  const resumeExisting = async (id: number) => {
    if (busy) return
    setResuming(true)
    try {
      const schema = await requireAddonSchema()
      if (!schema) {
        setPublishError('This source is unavailable.')
        return
      }
      const result = await getAddonValues(
        { type: 'submission', id, kind: 'update', name: addon.name },
        schema
      )
      if (result.status !== 'ok') {
        setPublishError(result.message)
        return
      }
      setSubmissionId(id)
      setForm(valuesToForm(result.values))
      setValidated(false)
      setValidationError(false)
      setFieldErrors({})
      setPublishError(null)
      setEditable(true)
    } finally {
      setResuming(false)
    }
  }

  const handleValidate = async () => {
    if (busy) return
    setValidating(true)
    setPublishError(null)
    setFieldErrors({})
    setValidationError(false)
    let validationPassed = false
    try {
      const result = await validateAddon(form, submissionId)
      if (result.status === 'not_open') {
        setEditable(false)
        setSubmissionId(null)
        setPublishError('This submission is no longer open.')
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

  const handleSubmit = async () => {
    if (busy || !validated) return
    setSaving(true)
    setPublishError(null)
    setFieldErrors({})
    try {
      const result = await submitAddon(form, submissionId)
      if (result.status === 'submitted') {
        const created = submissionId === null
        toast({
          title: created ? 'Addon submitted' : 'Submission updated',
          description: created ? "It's now in review." : 'Your changes were saved.',
        })
        onCancel()
        onSubmitted?.()
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
        setEditable(false)
        setSubmissionId(null)
        setPublishError('This submission is no longer open.')
        return
      }
      setPublishError(result.message)
    } catch {
      setPublishError("Couldn't publish this addon.")
    } finally {
      setSaving(false)
    }
  }
  return (
    <div className="space-y-6">
      <div>
        <h3 className="font-medium">{introTitle}</h3>
        <p className="mt-1 text-sm text-muted-foreground">{introNote}</p>
      </div>
      <AddonDeclarationFields
        form={form}
        setField={setField}
        fieldErrors={fieldErrors}
        busy={busy}
        lockedFields={nameLocked ? ['name'] : []}
      />
      {failCopy && <p className="text-sm text-destructive">{failCopy}</p>}
      <div className="flex flex-wrap gap-2 border-t pt-4">
        <div className="relative">
          {validated ? (
            <span
              className="publish-cta-ring pointer-events-none absolute inset-0 rounded-md"
              aria-hidden
            />
          ) : null}
          <Button
            disabled={ctaDisabled}
            onClick={() => {
              if (validated) {
                void handleSubmit()
                return
              }
              void handleValidate()
            }}
            className={cn(
              validated &&
                'bg-emerald-600 text-emerald-50 hover:bg-emerald-700 focus-visible:border-emerald-600 focus-visible:ring-emerald-400/50 publish-cta-scale'
            )}
          >
            {saving ? (
              <>
                <LoaderCircle className="animate-spin" />
                {submitLabel}
              </>
            ) : validating ? (
              <>
                <LoaderCircle className="animate-spin" />
                Validate
              </>
            ) : validated ? (
              <>
                <Check />
                {submitLabel}
              </>
            ) : (
              'Validate'
            )}
          </Button>
        </div>
        <Button type="button" variant="outline" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
      </div>
      <AlertDialog
        open={alreadyOpenId !== null}
        onOpenChange={open => {
          if (!open) setAlreadyOpenId(null)
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Submission already exists</AlertDialogTitle>
            <AlertDialogDescription>
              You already have an open or rejected submission for this name.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setAlreadyOpenId(null)}>
              Keep editing
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                const id = alreadyOpenId
                setAlreadyOpenId(null)
                if (id === null) return
                void resumeExisting(id)
              }}
            >
              Resume existing
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
