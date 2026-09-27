import { ArrowLeft, Check, CircleAlert, Code2, LoaderCircle } from 'lucide-react'
import { useRef, useState } from 'react'

import { AddonDeclarationFields } from '@/components/developer/AddonDeclarationFields'
import { AlreadyOpenDialog } from '@/components/developer/AlreadyOpenDialog'
import {
  INITIAL_PUBLISH_FORM,
  isPublishFormDirty,
  type PublishFormState,
} from '@/components/developer/constants'
import { useDeclarationForm } from '@/components/developer/useDeclarationForm'
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
  const [nameLocked, setNameLocked] = useState(false)
  const [discardOpen, setDiscardOpen] = useState(false)
  const declaration = useDeclarationForm({
    initial,
    resumeSource: id => ({ type: 'submission', id, kind: 'new', name: '' }),
    onResumed: kind => setNameLocked(kind === 'update'),
    onSubmitted: onClose,
    scrollContainer: mainRef,
  })
  const { form, schema, submissionId, busy, validated, validating, submitting, failCopy } =
    declaration

  const handleBack = () => {
    if (isPublishFormDirty(form, initial)) {
      setDiscardOpen(true)
      return
    }
    onClose()
  }

  const failHeader = failCopy !== null
  const successHeader = validated && declaration.publishError === null

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
          {schema ? (
            <AddonDeclarationFields
              schema={schema}
              form={form}
              setValue={declaration.setValue}
              setField={declaration.setField}
              fieldErrors={declaration.fieldErrors}
              busy={busy}
              lockedFields={nameLocked ? ['name'] : []}
            />
          ) : (
            <p className="text-sm text-muted-foreground">{declaration.schemaError ?? 'Loading…'}</p>
          )}
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
            onClick={declaration.primaryAction}
            className={cn(
              'w-32',
              validated &&
                'bg-emerald-600 text-emerald-50 hover:bg-emerald-700 focus-visible:border-emerald-600 focus-visible:ring-emerald-400/50 publish-cta-scale'
            )}
          >
            {submitting ? (
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

      <AlreadyOpenDialog
        open={declaration.alreadyOpen}
        onDismiss={declaration.dismissAlreadyOpen}
        onResume={declaration.resumeAlreadyOpen}
      />
    </div>
  )
}
