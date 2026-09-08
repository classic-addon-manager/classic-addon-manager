import { Browser } from '@wailsio/runtime'
import { CircleAlert, GithubIcon } from 'lucide-react'
import { type ReactNode, useState } from 'react'

import { APPROVED_TAGS, type PublishFormState } from '@/components/developer/constants'
import { DependenciesCombobox } from '@/components/developer/DependenciesCombobox'
import { KeywordsInput } from '@/components/developer/KeywordsInput'
import type { FieldErrors } from '@/components/developer/validate'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverAnchor, PopoverContent } from '@/components/ui/popover'
import { Textarea } from '@/components/ui/textarea'
import { Toggle } from '@/components/ui/toggle'
import { cn } from '@/lib/utils'

export type SetDeclarationField = <K extends keyof PublishFormState>(
  key: K,
  value: PublishFormState[K] | ((prev: PublishFormState[K]) => PublishFormState[K])
) => void

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

export function AddonDeclarationFields({
  form,
  setField,
  fieldErrors,
  busy,
  lockedFields = [],
}: {
  form: PublishFormState
  setField: SetDeclarationField
  fieldErrors: FieldErrors
  busy: boolean
  lockedFields?: readonly (keyof PublishFormState)[]
}) {
  const nameLocked = lockedFields.includes('name')

  return (
    <div className="space-y-8">
      <Section title="Identity">
        <div className="space-y-4 px-4 py-4">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <Field
              id="addon-name"
              label="Name"
              hint={
                nameLocked ? "Name can't be changed after publish." : 'No spaces. Starts with A–Z.'
              }
              error={fieldErrors.name}
            >
              <Input
                id="addon-name"
                value={form.name}
                disabled={busy || nameLocked}
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
            hint="Space-separated. No spaces inside a keyword."
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
  )
}
