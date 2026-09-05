import { Browser } from '@wailsio/runtime'
import {
  AlertTriangleIcon,
  ArrowLeft,
  Check,
  CheckIcon,
  CircleAlert,
  Code2,
  GithubIcon,
  LoaderCircle,
} from 'lucide-react'
import { type ReactNode, useRef, useState } from 'react'

import {
  APPROVED_TAGS,
  INITIAL_PUBLISH_FORM,
  isPublishFormDirty,
  type PublishFormState,
} from '@/components/developer/constants'
import { DependenciesCombobox } from '@/components/developer/DependenciesCombobox'
import { KeywordsInput } from '@/components/developer/KeywordsInput'
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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverAnchor, PopoverContent } from '@/components/ui/popover'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast'
import { Toggle } from '@/components/ui/toggle'
import { cn } from '@/lib/utils'

const Section = ({
  title,
  description,
  children,
}: {
  title: string
  description?: string
  children: ReactNode
}) => (
  <section className="space-y-3">
    <div className="px-1">
      <h2 className="text-sm font-medium tracking-tight">{title}</h2>
      {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
    </div>
    <div className="overflow-hidden rounded-xl border border-border/60 bg-card/40">{children}</div>
  </section>
)

const Field = ({
  id,
  label,
  hint,
  error,
  errorMode = 'focus',
  children,
}: {
  id?: string
  label: string
  hint?: ReactNode
  error?: string[]
  errorMode?: 'focus' | 'label'
  children: ReactNode
}) => {
  const [active, setActive] = useState(false)
  const hasError = !!error?.length
  const showOnLabel = errorMode === 'label'
  const open = hasError && active

  const errorPopover = (
    <PopoverContent
      side="top"
      align="start"
      sideOffset={8}
      className="pointer-events-none w-auto max-w-xs p-3"
      onOpenAutoFocus={event => event.preventDefault()}
      onCloseAutoFocus={event => event.preventDefault()}
    >
      <div className="flex gap-2">
        <CircleAlert className="mt-0.5 size-3.5 shrink-0 text-destructive" />
        <div className="space-y-1">
          {error?.map((message, index) => (
            <p key={`${message}-${index}`} className="text-xs font-medium text-destructive">
              {message}
            </p>
          ))}
        </div>
      </div>
    </PopoverContent>
  )

  return (
    <div className="space-y-1.5">
      <div className="flex items-center gap-1.5">
        <Label htmlFor={id}>{label}</Label>
        {showOnLabel && hasError && (
          <Popover modal={false} open={open}>
            <PopoverAnchor asChild>
              <button
                type="button"
                className="cursor-pointer text-destructive"
                aria-label="Validation error"
                onMouseEnter={() => setActive(true)}
                onMouseLeave={() => setActive(false)}
                onFocus={() => setActive(true)}
                onBlur={() => setActive(false)}
              >
                <CircleAlert className="size-3.5" />
              </button>
            </PopoverAnchor>
            {errorPopover}
          </Popover>
        )}
      </div>
      {showOnLabel ? (
        children
      ) : (
        <Popover modal={false} open={open}>
          <PopoverAnchor asChild>
            <div
              onFocusCapture={() => setActive(true)}
              onBlurCapture={event => {
                const next = event.relatedTarget
                if (next instanceof Node && event.currentTarget.contains(next)) return
                setActive(false)
              }}
            >
              {children}
            </div>
          </PopoverAnchor>
          {errorPopover}
        </Popover>
      )}
      {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  )
}

interface PublishAddonFormProps {
  onClose: () => void
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

export const PublishAddonForm = ({ onClose }: PublishAddonFormProps) => {
  const mainRef = useRef<HTMLElement>(null)
  const [form, setForm] = useState<PublishFormState>(INITIAL_PUBLISH_FORM)
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
    if (isPublishFormDirty(form)) {
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
          description: 'A catalog pull request was opened.',
          icon: CheckIcon,
          button: result.htmlUrl.startsWith('https://github.com/')
            ? {
                label: 'View PR',
                onClick: () => {
                  void Browser.OpenURL(result.htmlUrl)
                },
              }
            : undefined,
        })
        onClose()
        return
      }
      if (result.status === 'already_open') {
        toast({
          title: 'Submission already open',
          description: 'A catalog pull request for this addon already exists.',
          icon: AlertTriangleIcon,
          button: result.htmlUrl.startsWith('https://github.com/')
            ? {
                label: 'View PR',
                onClick: () => {
                  void Browser.OpenURL(result.htmlUrl)
                },
              }
            : undefined,
        })
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
          <Section title="Identity">
            <div className="space-y-4 px-4 py-4">
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <Field
                  id="addon-name"
                  label="Name"
                  hint="No spaces. Starts with A–Z."
                  error={fieldErrors.name}
                >
                  <Input
                    id="addon-name"
                    value={form.name}
                    disabled={busy}
                    aria-invalid={!!fieldErrors.name}
                    onChange={event => setField('name', event.target.value)}
                  />
                </Field>
                <Field id="addon-alias" label="Alias" error={fieldErrors.alias}>
                  <Input
                    id="addon-alias"
                    value={form.alias}
                    disabled={busy}
                    aria-invalid={!!fieldErrors.alias}
                    onChange={event => setField('alias', event.target.value)}
                  />
                </Field>
              </div>
              <Field id="addon-description" label="Description" error={fieldErrors.description}>
                <Textarea
                  id="addon-description"
                  value={form.description}
                  disabled={busy}
                  aria-invalid={!!fieldErrors.description}
                  onChange={event => setField('description', event.target.value)}
                />
              </Field>
              <Field id="addon-author" label="Author" error={fieldErrors.author}>
                <Input
                  id="addon-author"
                  value={form.author}
                  disabled={busy}
                  aria-invalid={!!fieldErrors.author}
                  onChange={event => setField('author', event.target.value)}
                />
              </Field>
            </div>
          </Section>

          <Section title="Repository" description="GitHub user/repo and branch.">
            <div className="space-y-4 px-4 py-4">
              <div className="grid gap-4 sm:grid-cols-[1fr_10rem]">
                <Field
                  id="addon-repo"
                  label="Repository"
                  hint="Format: user/repo"
                  error={fieldErrors.repo}
                >
                  <div
                    className={cn(
                      'border-input dark:bg-input/30 focus-within:border-ring focus-within:ring-ring/50 flex h-9 overflow-hidden rounded-md border shadow-xs focus-within:ring-[3px]',
                      fieldErrors.repo &&
                        'border-destructive focus-within:border-destructive focus-within:ring-destructive/20'
                    )}
                  >
                    <span className="text-muted-foreground flex shrink-0 items-center gap-1.5 border-r border-input px-2.5 text-sm">
                      <GithubIcon className="size-3.5" />
                      github.com/
                    </span>
                    <Input
                      id="addon-repo"
                      value={form.repo}
                      placeholder="user/repo"
                      disabled={busy}
                      aria-invalid={!!fieldErrors.repo}
                      onChange={event => setField('repo', event.target.value)}
                      className="h-full rounded-none border-0 bg-transparent shadow-none focus-visible:ring-0 dark:bg-transparent"
                    />
                  </div>
                </Field>
                <Field id="addon-branch" label="Branch" error={fieldErrors.branch}>
                  <Input
                    id="addon-branch"
                    value={form.branch}
                    placeholder="main"
                    disabled={busy}
                    aria-invalid={!!fieldErrors.branch}
                    onChange={event => setField('branch', event.target.value)}
                  />
                </Field>
              </div>
            </div>
          </Section>

          <Section title="Catalog">
            <div className="space-y-4 px-4 py-4">
              <Field label="Tags" hint="Pick up to 3." error={fieldErrors.tags} errorMode="label">
                <div
                  id="addon-tags"
                  role="group"
                  aria-label="Tags"
                  aria-invalid={fieldErrors.tags ? true : undefined}
                  className="flex flex-wrap gap-1.5"
                >
                  {APPROVED_TAGS.map(tag => {
                    const selected = form.tags.includes(tag)
                    return (
                      <Toggle
                        key={tag}
                        size="sm"
                        variant="outline"
                        pressed={selected}
                        disabled={busy || (!selected && form.tags.length >= 3)}
                        onPressedChange={pressed => {
                          setField('tags', prev => {
                            if (pressed) {
                              if (prev.includes(tag) || prev.length >= 3) return prev
                              return [...prev, tag]
                            }
                            return prev.filter(item => item !== tag)
                          })
                        }}
                        className="px-2.5 data-[state=on]:bg-primary data-[state=on]:text-primary-foreground"
                      >
                        {tag}
                      </Toggle>
                    )
                  })}
                </div>
              </Field>
              <Field
                id="addon-keywords"
                label="Keywords"
                hint="Space-separated. No spaces inside a token."
                error={fieldErrors.keywords}
              >
                <KeywordsInput
                  id="addon-keywords"
                  value={form.keywords}
                  invalid={!!fieldErrors.keywords}
                  disabled={busy}
                  onChange={keywords => setField('keywords', keywords)}
                />
              </Field>
            </div>
          </Section>

          <Section title="Optional">
            <div className="space-y-4 px-4 py-4">
              <Field
                label="Dependencies"
                hint="Selected addons will be installed alongside your addon."
                error={fieldErrors.dependencies}
              >
                <div id="addon-dependencies">
                  <DependenciesCombobox
                    selected={form.dependencies}
                    invalid={!!fieldErrors.dependencies}
                    disabled={busy}
                    onChange={names => setField('dependencies', names)}
                  />
                </div>
              </Field>
              <Field
                id="addon-kofi"
                label="Ko-fi username"
                hint={
                  <>
                    Players can support you at{' '}
                    <button
                      type="button"
                      className="cursor-pointer text-primary underline-offset-2 hover:underline"
                      onClick={() =>
                        Browser.OpenURL(
                          form.kofi.trim()
                            ? `https://ko-fi.com/${form.kofi.trim()}`
                            : 'https://ko-fi.com'
                        )
                      }
                    >
                      {form.kofi.trim() ? `ko-fi.com/${form.kofi.trim()}` : 'ko-fi.com'}
                    </button>
                    .
                  </>
                }
                error={fieldErrors.kofi}
              >
                <Input
                  id="addon-kofi"
                  value={form.kofi}
                  disabled={busy}
                  aria-invalid={!!fieldErrors.kofi}
                  onChange={event => setField('kofi', event.target.value)}
                />
              </Field>
            </div>
          </Section>
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
