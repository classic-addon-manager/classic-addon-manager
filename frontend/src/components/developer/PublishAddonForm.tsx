import {
  AlertTriangleIcon,
  ArrowLeft,
  Check,
  CheckIcon,
  CircleAlert,
  Code2,
  LoaderCircle,
} from 'lucide-react'
import { useRef, useState } from 'react'

import { AddonDeclarationFields } from '@/components/developer/AddonDeclarationFields'
import {
  INITIAL_PUBLISH_FORM,
  isPublishFormDirty,
  type PublishFormState,
} from '@/components/developer/constants'
import {
  type FieldErrors,
  publishFormToValidatePayload,
  submitAddon,
  validateAddon,
} from '@/components/developer/validate'
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

const FORM_FIELD_ORDER: (keyof PublishFormState)[] = [
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

function scrollFirstFieldErrorIntoView(main: HTMLElement | null, errors: FieldErrors) {
  if (!main) return
  const firstKey = FORM_FIELD_ORDER.find(key => !!errors[key]?.length)
  if (!firstKey) return
  const target = document.getElementById(`addon-${firstKey}`)
  if (!target) return

  const mainRect = main.getBoundingClientRect()
  const targetRect = target.getBoundingClientRect()
  const fullyVisible = targetRect.top >= mainRect.top && targetRect.bottom <= mainRect.bottom
  if (fullyVisible) return

  target.scrollIntoView({ behavior: 'smooth', block: 'center' })
}

export const PublishAddonForm = ({
  onClose,
  initial = INITIAL_PUBLISH_FORM,
}: PublishAddonFormProps) => {
  const mainRef = useRef<HTMLElement>(null)
  const [form, setForm] = useState<PublishFormState>(initial)
  const [discardOpen, setDiscardOpen] = useState(false)
  const [validating, setValidating] = useState(false)
  const [validated, setValidated] = useState(false)
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})
  const [validationError, setValidationError] = useState(false)
  const [publishing, setPublishing] = useState(false)
  const [publishError, setPublishError] = useState<string | null>(null)
  const busy = validating || publishing

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
    setFieldErrors(prev => {
      if (!prev[key]) return prev
      const next = { ...prev }
      delete next[key]
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
        scrollFirstFieldErrorIntoView(mainRef.current, fields)
      })
      return
    }
    setValidationError(true)
  }

  const handleValidate = async () => {
    if (validating) return
    setValidating(true)
    setPublishError(null)
    setFieldErrors({})
    setValidationError(false)
    let validationPassed = false
    try {
      const result = await validateAddon(publishFormToValidatePayload(form))
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
      const result = await submitAddon(publishFormToValidatePayload(form))
      if (result.status === 'submitted') {
        toast({
          title: 'Addon submitted',
          description: "It's now in review.",
          icon: CheckIcon,
        })
        onClose()
        return
      }
      if (result.status === 'already_open') {
        toast({
          title: 'Submission already open',
          description: 'This addon is already in review.',
          icon: AlertTriangleIcon,
        })
        onClose()
        return
      }
      if (result.status === 'invalid') {
        setValidated(false)
        applyInvalidResult(result.fields)
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
              <h1 className="text-xl font-semibold tracking-tight">Publish addon</h1>
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
                Publish
              </>
            ) : validating ? (
              <>
                <LoaderCircle className="animate-spin" />
                Validate
              </>
            ) : validated ? (
              <>
                <Check />
                Publish
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
    </div>
  )
}
