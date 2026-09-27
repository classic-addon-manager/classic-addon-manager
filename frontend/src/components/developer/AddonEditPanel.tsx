import { Check, LoaderCircle } from 'lucide-react'

import { AddonDeclarationFields } from '@/components/developer/AddonDeclarationFields'
import { AlreadyOpenDialog } from '@/components/developer/AlreadyOpenDialog'
import { publishFormFromPayload, type PublishFormState } from '@/components/developer/constants'
import type { OwnedAddon } from '@/components/developer/ownedParse'
import { useDeclarationForm } from '@/components/developer/useDeclarationForm'
import { Button } from '@/components/ui/button'
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
      library: addon.library,
      kofi: '',
    })
  const declaration = useDeclarationForm({
    initial,
    initialSubmissionId,
    resumeSource: id => ({ type: 'submission', id, kind: 'update', name: addon.name }),
    onSubmitted: () => {
      onCancel()
      onSubmitted?.()
    },
  })
  const { form, schema, busy, validated, validating, submitting, failCopy } = declaration
  const ctaDisabled = busy || (!validated && requireChanges && !declaration.dirty)
  return (
    <div className="space-y-6">
      <div>
        <h3 className="font-medium">{introTitle}</h3>
        <p className="mt-1 text-sm text-muted-foreground">{introNote}</p>
      </div>
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
            onClick={declaration.primaryAction}
            className={cn(
              validated &&
                'bg-emerald-600 text-emerald-50 hover:bg-emerald-700 focus-visible:border-emerald-600 focus-visible:ring-emerald-400/50 publish-cta-scale'
            )}
          >
            {submitting ? (
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
      <AlreadyOpenDialog
        open={declaration.alreadyOpen}
        onDismiss={declaration.dismissAlreadyOpen}
        onResume={declaration.resumeAlreadyOpen}
      />
    </div>
  )
}
