import { Browser } from '@wailsio/runtime'

export function DetailField({
  label,
  value,
  href,
}: {
  label: string
  value: string
  href?: string
}) {
  return (
    <div className="flex flex-wrap justify-between gap-x-4 gap-y-1 py-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 wrap-break-word">
        {value && href ? (
          <button
            type="button"
            className="cursor-pointer text-left wrap-break-word text-primary underline-offset-4 hover:underline"
            onClick={() => void Browser.OpenURL(href)}
          >
            {value}
          </button>
        ) : (
          value || 'None'
        )}
      </dd>
    </div>
  )
}
