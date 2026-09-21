import { Browser } from '@wailsio/runtime'
import { CircleAlert, GithubIcon, ImageIcon } from 'lucide-react'
import { type ReactNode, useState } from 'react'

import type { PublishFormState } from '@/components/developer/constants'
import { uploadIcon } from '@/components/developer/declarationApi'
import { DependenciesCombobox } from '@/components/developer/DependenciesCombobox'
import { listOf, textOf } from '@/components/developer/formValues.ts'
import { IconPickerDialog } from '@/components/developer/IconPickerDialog'
import { KeywordsInput } from '@/components/developer/KeywordsInput'
import type { AddonSchema, SchemaField, WireValue } from '@/components/developer/types.ts'
import type { FieldErrors } from '@/components/developer/validate'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverAnchor, PopoverContent } from '@/components/ui/popover'
import { Textarea } from '@/components/ui/textarea'
import { Toggle } from '@/components/ui/toggle'
import { cn } from '@/lib/utils'

export type SetDeclarationField = <K extends 'iconAssetId' | 'iconUrl'>(
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

function IconFieldRow({
  iconUrl,
  error,
  busy,
  onChange,
}: {
  iconUrl: string | null
  error?: string[]
  busy: boolean
  onChange: (assetId: string | null, url: string | null) => void
}) {
  const [pickerOpen, setPickerOpen] = useState(false)

  return (
    <Field
      label="Icon"
      hint="PNG only · 1:1 · min 50×50 · max 3 MB"
      error={error}
      errorMode="label"
    >
      <div id="addon-icon" className="flex items-center gap-3">
        {iconUrl ? (
          <img alt="" className="size-12 rounded-md border object-cover" src={iconUrl} />
        ) : (
          <div className="flex size-12 items-center justify-center rounded-md border bg-card text-muted-foreground">
            <ImageIcon className="size-5" />
          </div>
        )}
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={busy}
          onClick={() => setPickerOpen(true)}
        >
          Change icon
        </Button>
      </div>
      <IconPickerDialog
        open={pickerOpen}
        onOpenChange={setPickerOpen}
        currentIconUrl={iconUrl}
        onSave={async selected => {
          if (selected === null) {
            onChange('', null)
            return null
          }
          const result = await uploadIcon(selected)
          if (result.status === 'error') return result.message
          onChange(result.assetId, result.url)
          return null
        }}
      />
    </Field>
  )
}

export type SetDeclarationValue = (
  key: string,
  value: WireValue | ((prev: WireValue | undefined) => WireValue)
) => void

const SECTION_DESCRIPTIONS: Record<string, string> = {
  Repository: 'GitHub user/repo and branch.',
}

export function AddonDeclarationFields({
  schema,
  form,
  setValue,
  setField,
  fieldErrors,
  busy,
  lockedFields = [],
}: {
  schema: AddonSchema
  form: PublishFormState
  setValue: SetDeclarationValue
  setField: SetDeclarationField
  fieldErrors: FieldErrors
  busy: boolean
  lockedFields?: readonly string[]
}) {
  const fieldByKey = new Map(schema.fields.map(field => [field.key, field]))
  const sections = new Map<string, SchemaField[]>()
  for (const field of schema.fields) {
    if (field.key === 'branch' && fieldByKey.has('repo')) continue
    const list = sections.get(field.section)
    if (list) list.push(field)
    else sections.set(field.section, [field])
  }
  const branchField = fieldByKey.get('branch')
  const renderField = (field: SchemaField): ReactNode => {
    switch (field.key) {
      case 'repo':
        return (
          <RepoRow
            field={field}
            branch={branchField}
            form={form}
            setValue={setValue}
            fieldErrors={fieldErrors}
            busy={busy}
          />
        )
      case 'dependencies':
        return (
          <Field label={field.label} hint={field.hint} error={fieldErrors.dependencies}>
            <div id="addon-dependencies">
              <DependenciesCombobox
                selected={listOf(form.values, 'dependencies')}
                invalid={!!fieldErrors.dependencies}
                disabled={busy}
                onChange={names => setValue('dependencies', names)}
              />
            </div>
          </Field>
        )
      case 'kofi':
        return (
          <KofiField
            field={field}
            form={form}
            setValue={setValue}
            fieldErrors={fieldErrors}
            busy={busy}
          />
        )
      default:
        return (
          <GenericField
            field={field}
            form={form}
            setValue={setValue}
            fieldErrors={fieldErrors}
            busy={busy}
            lockedFields={lockedFields}
          />
        )
    }
  }
  const iconField = (
    <IconFieldRow
      key="icon"
      iconUrl={form.iconUrl}
      error={fieldErrors.icon}
      busy={busy}
      onChange={(assetId, url) => {
        setField('iconAssetId', assetId)
        setField('iconUrl', url)
      }}
    />
  )

  return (
    <div className="space-y-8">
      {[...sections.entries()].map(([section, fields]) => {
        const rows: ReactNode[] = []
        for (let i = 0; i < fields.length; i++) {
          const field = fields[i]
          const next = fields[i + 1]
          if (field.key !== 'repo' && field.half && next?.half) {
            rows.push(
              <div
                key={`${field.key}+${next.key}`}
                className="grid grid-cols-1 gap-4 sm:grid-cols-2"
              >
                {renderField(field)}
                {renderField(next)}
              </div>
            )
            if (field.key === 'alias' || next.key === 'alias') rows.push(iconField)
            i++
            continue
          }
          rows.push(<div key={field.key}>{renderField(field)}</div>)
          if (field.key === 'alias') rows.push(iconField)
        }
        return (
          <Section key={section} title={section} description={SECTION_DESCRIPTIONS[section]}>
            <div className="space-y-4 px-4 py-4">{rows}</div>
          </Section>
        )
      })}
    </div>
  )
}

function GenericField({
  field,
  form,
  setValue,
  fieldErrors,
  busy,
  lockedFields,
}: {
  field: SchemaField
  form: PublishFormState
  setValue: SetDeclarationValue
  fieldErrors: FieldErrors
  busy: boolean
  lockedFields: readonly string[]
}) {
  const id = `addon-${field.key}`
  const error = fieldErrors[field.key]
  const locked = !!field.immutable && lockedFields.includes(field.key)
  const hint = locked ? `${field.label} can't be changed after publish.` : field.hint

  if (field.widget === 'checkbox') {
    return (
      <div className="space-y-1.5">
        <div className="flex items-center gap-2">
          <Checkbox
            id={id}
            checked={form.values[field.key] === true}
            disabled={busy || locked}
            aria-invalid={!!error}
            onCheckedChange={checked => setValue(field.key, checked === true)}
          />
          <Label htmlFor={id} className="cursor-pointer font-normal">
            {field.label}
          </Label>
        </div>
        {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
        {error?.map((message, index) => (
          <p key={index} className="text-xs font-medium text-destructive">
            {message}
          </p>
        ))}
      </div>
    )
  }

  if (field.widget === 'enum-multi') {
    const selected = listOf(form.values, field.key)
    const options = field.enum ?? []
    const max = field.maxItems ?? 0
    return (
      <Field label={field.label} hint={hint} error={error} errorMode="label">
        <div
          id={id}
          role="group"
          aria-label={field.label}
          aria-invalid={error ? true : undefined}
          className="flex flex-wrap gap-1.5"
        >
          {options.map(option => {
            const on = selected.includes(option)
            return (
              <Toggle
                key={option}
                size="sm"
                variant="outline"
                pressed={on}
                disabled={busy || locked || (!on && max > 0 && selected.length >= max)}
                onPressedChange={pressed => {
                  setValue(field.key, prev => {
                    const list = Array.isArray(prev) ? prev : []
                    if (pressed) {
                      if (list.includes(option) || (max > 0 && list.length >= max)) return list
                      return [...list, option]
                    }
                    return list.filter(item => item !== option)
                  })
                }}
                className="px-2.5 data-[state=on]:bg-primary data-[state=on]:text-primary-foreground"
              >
                {option}
              </Toggle>
            )
          })}
        </div>
      </Field>
    )
  }

  if (field.widget === 'string-list') {
    return (
      <Field id={id} label={field.label} hint={hint} error={error}>
        <KeywordsInput
          id={id}
          value={listOf(form.values, field.key)}
          invalid={!!error}
          disabled={busy || locked}
          onChange={items => setValue(field.key, items)}
        />
      </Field>
    )
  }

  return (
    <Field id={id} label={field.label} hint={hint} error={error}>
      {field.widget === 'textarea' ? (
        <Textarea
          id={id}
          value={textOf(form.values, field.key)}
          disabled={busy || locked}
          aria-invalid={!!error}
          onChange={event => setValue(field.key, event.target.value)}
        />
      ) : (
        <Input
          id={id}
          value={textOf(form.values, field.key)}
          disabled={busy || locked}
          aria-invalid={!!error}
          onChange={event => setValue(field.key, event.target.value)}
        />
      )}
    </Field>
  )
}

function RepoRow({
  field,
  branch,
  form,
  setValue,
  fieldErrors,
  busy,
}: {
  field: SchemaField
  branch: SchemaField | undefined
  form: PublishFormState
  setValue: SetDeclarationValue
  fieldErrors: FieldErrors
  busy: boolean
}) {
  return (
    <div className="grid gap-4 sm:grid-cols-[1fr_10rem]">
      <Field id="addon-repo" label={field.label} hint={field.hint} error={fieldErrors.repo}>
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
            value={textOf(form.values, 'repo')}
            placeholder="user/repo"
            disabled={busy}
            aria-invalid={!!fieldErrors.repo}
            onChange={event => setValue('repo', event.target.value)}
            className="h-full rounded-none border-0 bg-transparent shadow-none focus-visible:ring-0 dark:bg-transparent"
          />
        </div>
      </Field>
      {branch ? (
        <Field id="addon-branch" label={branch.label} error={fieldErrors.branch}>
          <Input
            id="addon-branch"
            value={textOf(form.values, 'branch')}
            placeholder="main"
            disabled={busy}
            aria-invalid={!!fieldErrors.branch}
            onChange={event => setValue('branch', event.target.value)}
          />
        </Field>
      ) : null}
    </div>
  )
}

function KofiField({
  field,
  form,
  setValue,
  fieldErrors,
  busy,
}: {
  field: SchemaField
  form: PublishFormState
  setValue: SetDeclarationValue
  fieldErrors: FieldErrors
  busy: boolean
}) {
  const kofi = textOf(form.values, 'kofi')
  return (
    <Field
      id="addon-kofi"
      label={`${field.label} username`}
      hint={
        <>
          Players can support you at{' '}
          <button
            type="button"
            className="cursor-pointer text-primary underline-offset-2 hover:underline"
            onClick={() =>
              Browser.OpenURL(
                kofi.trim() ? `https://ko-fi.com/${kofi.trim()}` : 'https://ko-fi.com'
              )
            }
          >
            {kofi.trim() ? `ko-fi.com/${kofi.trim()}` : 'ko-fi.com'}
          </button>
          .
        </>
      }
      error={fieldErrors.kofi}
    >
      <Input
        id="addon-kofi"
        value={kofi}
        disabled={busy}
        aria-invalid={!!fieldErrors.kofi}
        onChange={event => setValue('kofi', event.target.value)}
      />
    </Field>
  )
}
