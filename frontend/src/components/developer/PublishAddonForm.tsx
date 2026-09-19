import { ArrowLeft, Check, CheckIcon, CircleAlert, Code2, LoaderCircle } from 'lucide-react'
import { useRef, useState } from 'react'

import { AddonDeclarationFields } from '@/components/developer/AddonDeclarationFields'
import {
  INITIAL_PUBLISH_FORM,
  isPublishFormDirty,
  type PublishFormState,
} from '@/components/developer/constants'
import {
  FORM_FIELD_ORDER,
  scrollFirstFieldErrorIntoView,
} from '@/components/developer/declarationFields'
import { valuesToForm } from '@/components/developer/formValues.ts'
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

interface PublishAddonFormProps {
  onClose: () => void
  initial?: PublishFormState
}

export const PublishAddonForm = ({
  onClose,
  initial = INITIAL_PUBLISH_FORM,
}: PublishAddonFormProps) => {
  const mainRef = useRef<HTMLElement>(null)
  const [form, setForm] = useState<PublishFormState>(initial)
  const [submissionId, setSubmissionId] = useState<number | null>(null)
  const [nameLocked, setNameLocked] = useState(false)
  const [editable, setEditable] = useState(true)
  const [alreadyOpenId, setAlreadyOpenId] = useState<number | null>(null)
  const [discardOpen, setDiscardOpen] = useState(false)
  const [validating, setValidating] = useState(false)
  const [validated, setValidated] = useState(false)
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})
  const [validationError, setValidationError] = useState(false)
  const [publishing, setPublishing] = useState(false)
  const [resuming, setResuming] = useState(false)
  const [publishError, setPublishError] = useState<string | null>(null)
  const busy = validating || publishing || resuming || !editable

  const setField = <K extends keyof PublishFormState>(
    key: K,
    value: PublishFormState[K] | ((prev: PublishFormState[K]) => PublishFormState[K])
  ) => {
    setForm(prev => {
      const nextValue =
        typeof value === 'function'
          ? (value as (prev: PublishFormState[K]) => PublishFormState[K])(prev[key])
          : value
      if (Object.is(nextValue, prev[key])) return prev
      return { ...prev, [key]: nextValue }
    })
    setValidated(false)
    setValidationError(false)
    setFieldErrors(prev => {
      const errorKey = key === 'iconAssetId' || key === 'iconUrl' ? 'icon' : key
      if (!prev[errorKey]) return prev
      const next = { ...prev }
      delete next[errorKey]
      return next
    })
    setPublishError(null)
  }

  const handleBack = () => {
    if (isPublishFormDirty(form, initial)) {
      setDiscardOpen(true)
      return
    }
    onClose()
  }

  const applyInvalidResult = (fields: FieldErrors): void => {
    const hasFields = FORM_FIELD_ORDER.some(key => !!fields[key]?.length)
    if (hasFields) {
      setFieldErrors(fields)
      requestAnimationFrame(() => {
        scrollFirstFieldErrorIntoView(fields, mainRef.current)
      })
      return
    }
    setValidationError(true)
  }

  const resumeSubmission = async (id: number) => {
    if (busy) return
    setResuming(true)
    try {
      const schema = await requireAddonSchema()
      if (!schema) {
        setPublishError('This source is unavailable.')
        return
      }
      const result = await getAddonValues({ type: 'submission', id, kind: 'new', name: '' }, schema)
      if (result.status !== 'ok') {
        setPublishError(result.message)
        return
      }
      setSubmissionId(id)
      setNameLocked(result.kind === 'update')
      setForm(valuesToForm(result.values))
      setValidated(false)
      setValidationError(false)
      setFieldErrors({})
      setPublishError(null)
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

  const handlePublish = async () => {
    if (busy) return
    setPublishing(true)
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
        onClose()
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
      setPublishing(false)
    }
  }

  const handlePrimaryAction = () => {
    if (busy) return
    if (validated) {
      void handlePublish()
      return
    }
    void handleValidate()
  }

  const hasFieldErrors = FORM_FIELD_ORDER.some(key => !!fieldErrors[key]?.length)
  const failHeader = publishError !== null || (!validated && (hasFieldErrors || validationError))
  const successHeader = validated && publishError === null
  const failCopy = hasFieldErrors
    ? 'Fix the highlighted fields.'
    : publishError !== null
      ? publishError
      : "Couldn't validate this addon."

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header
        className={cn(
          'relative border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60',
          successHeader && 'border-emerald-500/20',
          failHeader && 'border-destructive/20'
        )}
      >
        {successHeader ? (
          <div className="pointer-events-none absolute inset-0 bg-emerald-500/10" aria-hidden />
        ) : failHeader ? (
          <div className="pointer-events-none absolute inset-0 bg-destructive/10" aria-hidden />
        ) : null}
        <div className="relative container flex h-16 items-center justify-between gap-4 px-4">
          <div className="flex min-w-0 items-center gap-3">
            <div
              className={cn(
                'rounded-lg p-2',
                successHeader && 'bg-emerald-500/20 publish-check-pop',
                failHeader && 'bg-destructive/20',
                !successHeader && !failHeader && 'bg-primary/10'
              )}
            >
              {successHeader ? (
                <Check className="h-6 w-6 text-emerald-400" />
              ) : failHeader ? (
                <CircleAlert className="h-6 w-6 text-destructive" />
              ) : (
                <Code2 className="h-6 w-6 text-primary" />
              )}
            </div>
            <div className="min-w-0">
              <h1 className="text-xl font-semibold tracking-tight">
                {submissionId === null ? 'Publish addon' : 'Update submission'}
              </h1>
              <p
                className={cn(
                  'text-sm',
                  successHeader && 'text-emerald-400',
                  failHeader && 'text-destructive',
                  !successHeader && !failHeader && 'text-muted-foreground'
                )}
              >
                {successHeader
                  ? 'Declaration valid - ready to publish.'
                  : failHeader
                    ? failCopy
                    : 'Fill in the addon declaration.'}
              </p>
            </div>
          </div>
          <Button
            type="button"
            variant="outline"
            className="w-32"
            disabled={busy}
            onClick={handleBack}
          >
            <ArrowLeft />
            Back
          </Button>
        </div>
      </header>

      <main ref={mainRef} className="min-h-0 flex-1 overflow-auto">
        <div className="container mx-auto max-w-2xl space-y-8 px-4 py-8">
          <AddonDeclarationFields
            form={form}
            setField={setField}
            fieldErrors={fieldErrors}
            busy={busy}
            lockedFields={nameLocked ? ['name'] : []}
          />
        </div>
      </main>

      <footer className="flex shrink-0 justify-end overflow-visible border-t bg-background/95 px-4 py-3">
        <div className="relative">
          {validated ? (
            <span
              key="publish-cta-ring"
              className="publish-cta-ring pointer-events-none absolute inset-0 rounded-md"
              aria-hidden
            />
          ) : null}
          <Button
            type="button"
            disabled={busy}
            onClick={handlePrimaryAction}
            className={cn(
              'w-32',
              validated &&
                'bg-emerald-600 text-emerald-50 hover:bg-emerald-700 focus-visible:border-emerald-600 focus-visible:ring-emerald-400/50 publish-cta-scale'
            )}
          >
            {publishing ? (
              <>
                <LoaderCircle className="animate-spin" />
                {submissionId === null ? 'Publish' : 'Update'}
              </>
            ) : validating ? (
              <>
                <LoaderCircle className="animate-spin" />
                Validate
              </>
            ) : validated ? (
              <>
                <Check />
                {submissionId === null ? 'Submit' : 'Update'}
              </>
            ) : (
              'Validate'
            )}
          </Button>
        </div>
      </footer>

      <AlertDialog open={discardOpen} onOpenChange={setDiscardOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Discard this addon?</AlertDialogTitle>
            <AlertDialogDescription>Your declaration will be lost.</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Stay</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={onClose}>
              Discard
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

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
                void resumeSubmission(id)
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
