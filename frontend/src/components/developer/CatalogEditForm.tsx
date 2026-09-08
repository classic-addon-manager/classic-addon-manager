import {
  catalogFieldLabels,
  type CatalogFields,
  changedCatalogFields,
} from '@/components/developer/catalogEditing'

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
