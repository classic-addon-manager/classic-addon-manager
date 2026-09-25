import { Info } from 'lucide-react'
import type { ReactNode } from 'react'

const steps: Array<ReactNode> = [
  'Open the Dashboard',
  'Select the addon from the list',
  'Open the menu in the details pane',
  <>
    Choose <span className="font-medium text-foreground">Report issue</span>
  </>,
]

export const ReportIssueGuide = () => (
  <>
    <ol className="space-y-2.5 px-4 py-3.5">
      {steps.map((step, i) => (
        <li key={i} className="flex items-center gap-2.5">
          <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-muted/60 text-[11px] font-medium tabular-nums text-muted-foreground">
            {i + 1}
          </span>
          <span className="text-sm text-foreground/90">{step}</span>
        </li>
      ))}
    </ol>
    <div className="flex items-center gap-2 border-t border-border/60 bg-muted/15 px-4 py-3">
      <Info className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
      <p className="text-xs text-muted-foreground">
        Available for managed addons only. Requires a GitHub account.
      </p>
    </div>
  </>
)
