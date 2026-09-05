import { useState } from 'react'

import {
  catalogFieldLabels,
  type CatalogFields,
  changedCatalogFields,
} from '@/components/developer/catalogEditing'
import { APPROVED_TAGS } from '@/components/developer/constants'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  initial: CatalogFields
  published: CatalogFields
  submitLabel: string
  onCancel: () => void
  onSubmit: (values: CatalogFields) => void
}

export function CatalogEditForm({ initial, published, submitLabel, onCancel, onSubmit }: Props) {
  // Deliberately local: unmounting on navigation abandons unsubmitted values.
  const [values, setValues] = useState<CatalogFields>(() => ({
    ...initial,
    tags: [...initial.tags],
  }))
  const [comparing, setComparing] = useState(false)
  const tags = [...new Set<string>([...APPROVED_TAGS, ...initial.tags])]

  if (comparing) {
    return (
      <div className="space-y-6">
        <div>
          <h3 className="font-medium">Review changes</h3>
          <p className="mt-1 text-sm text-muted-foreground">
            Your published listing stays unchanged until approval.
          </p>
        </div>
        <CatalogComparison published={published} proposed={values} />
        <div className="flex flex-wrap gap-2 border-t pt-4">
          <Button
            onClick={() => onSubmit(values)}
            disabled={changedCatalogFields(initial, values).length === 0}
          >
            {submitLabel}
          </Button>
          <Button variant="outline" onClick={() => setComparing(false)}>
            Back to editing
          </Button>
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
        </div>
      </div>
    )
  }

  return (
    <form
      className="space-y-5"
      onSubmit={event => {
        event.preventDefault()
        setValues({
          ...values,
          alias: values.alias.trim(),
          description: values.description.trim(),
          repo: values.repo.trim(),
          branch: values.branch.trim(),
        })
        setComparing(true)
      }}
    >
      <div>
        <h3 className="font-medium">Edit catalog details</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          Leaving this form discards unsent edits.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="catalog-alias">Display name</Label>
        <Input
          id="catalog-alias"
          value={values.alias}
          onChange={e => setValues({ ...values, alias: e.target.value })}
          required
          pattern=".*\S.*"
        />
      </div>
      <div className="space-y-2">
        <Label htmlFor="catalog-description">Description</Label>
        <Textarea
          id="catalog-description"
          value={values.description}
          onChange={e => setValues({ ...values, description: e.target.value })}
          className="min-h-28"
          required
        />
      </div>
      <fieldset className="space-y-2">
        <legend className="text-sm font-medium">Tags</legend>
        <div className="flex flex-wrap gap-2">
          {tags.map(tag => (
            <Button
              key={tag}
              type="button"
              size="sm"
              variant={values.tags.includes(tag) ? 'default' : 'outline'}
              aria-pressed={values.tags.includes(tag)}
              onClick={() =>
                setValues({
                  ...values,
                  tags: values.tags.includes(tag)
                    ? values.tags.filter(value => value !== tag)
                    : [...values.tags, tag],
                })
              }
            >
              {tag}
            </Button>
          ))}
        </div>
      </fieldset>
      <div className="space-y-2">
        <Label htmlFor="catalog-repo">Repository</Label>
        <Input
          id="catalog-repo"
          value={values.repo}
          onChange={e => setValues({ ...values, repo: e.target.value })}
          pattern="[^/\s]+/[^/\s]+"
          placeholder="owner/repository"
          required
          aria-describedby="catalog-repo-hint"
        />
        <p id="catalog-repo-hint" className="text-xs text-muted-foreground">
          owner/repository
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="catalog-branch">Branch</Label>
        <Input
          id="catalog-branch"
          value={values.branch}
          onChange={e => setValues({ ...values, branch: e.target.value })}
          required
          pattern=".*\S.*"
        />
      </div>
      {/* TODO(backend): Validate editable fields and repository/branch before opening a review. */}
      <div className="flex flex-wrap gap-2 border-t pt-4">
        <Button type="submit">Review changes</Button>
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </form>
  )
}

export function CatalogComparison({
  published,
  proposed,
}: {
  published: CatalogFields
  proposed: CatalogFields
}) {
  const changed = changedCatalogFields(published, proposed)
  if (changed.length === 0)
    return <p className="text-sm text-muted-foreground">No changes from the published listing.</p>
  return (
    <div className="space-y-5">
      {changed.map(key => (
        <section key={key} className="space-y-2">
          <h4 className="text-sm font-medium">{catalogFieldLabels[key]}</h4>
          {(['Published', 'Proposed'] as const).map(label => {
            const value = (label === 'Published' ? published : proposed)[key]
            return (
              <div key={label} className="rounded-lg border bg-card/40 p-3">
                <p
                  className={
                    label === 'Proposed' ? 'text-xs text-primary' : 'text-xs text-muted-foreground'
                  }
                >
                  {label}
                </p>
                <p className="mt-1 whitespace-pre-wrap break-words text-sm">
                  {(Array.isArray(value) ? value.join(', ') : value) || 'None'}
                </p>
              </div>
            )
          })}
        </section>
      ))}
    </div>
  )
}
