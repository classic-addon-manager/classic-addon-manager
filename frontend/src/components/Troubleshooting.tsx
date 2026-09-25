import { WrenchIcon } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'

import { Section } from '@/components/shared/Section'
import { AddonRecovery } from '@/components/troubleshooting/AddonRecovery'
import {
  type DiagnosticData,
  DiagnosticsPanel,
} from '@/components/troubleshooting/DiagnosticsPanel'
import { ReportIssueGuide } from '@/components/troubleshooting/ReportIssueGuide'
import { getErrorMessage, safeCall } from '@/lib/utils'
import type { LogParseResult } from '@/lib/wails'
import { LocalAddonService } from '@/lib/wails'

export const Troubleshooting = () => {
  const [diagnosticData, setDiagnosticData] = useState<DiagnosticData>({
    issueCount: 0,
    groupedIssues: {},
  })
  const [isLoadingDiagnostics, setIsLoadingDiagnostics] = useState(true)
  const [diagnosticsError, setDiagnosticsError] = useState('')
  const [diagnosticsRun, setDiagnosticsRun] = useState(0)
  const [hasDiagnostics, setHasDiagnostics] = useState(false)

  useEffect(() => {
    let cancelled = false

    const load = async () => {
      const startTime = Date.now()
      const [issues, err] = await safeCall(LocalAddonService.DiagnoseIssues())
      const remaining = diagnosticsRun > 0 ? 500 - (Date.now() - startTime) : 0
      if (remaining > 0) await new Promise(resolve => setTimeout(resolve, remaining))
      if (cancelled) return

      if (err) {
        console.error('Failed to run diagnostics:', err)
        setDiagnosticsError(getErrorMessage(err, 'Failed to run diagnostics'))
        setDiagnosticData({ issueCount: 0, groupedIssues: {} })
      } else {
        const issuesArray: LogParseResult[] = issues || []
        const groupedIssues: DiagnosticData['groupedIssues'] = {}
        for (const issue of issuesArray) {
          if (!groupedIssues[issue.Addon]) {
            groupedIssues[issue.Addon] = []
          }
          groupedIssues[issue.Addon].push({
            type: issue.Type,
            error: issue.Error,
            file: issue.File,
            count: issue.Count,
          })
        }
        setDiagnosticData({ issueCount: issuesArray.length, groupedIssues })
        setDiagnosticsError('')
      }
      setHasDiagnostics(true)
      setIsLoadingDiagnostics(false)
    }

    void load()
    return () => {
      cancelled = true
    }
  }, [diagnosticsRun])

  const runDiagnostics = useCallback(() => {
    setIsLoadingDiagnostics(true)
    setDiagnosticsRun(run => run + 1)
  }, [])

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
        <div className="container flex h-16 items-center gap-4 px-4">
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-primary/10 p-2">
              <WrenchIcon className="h-6 w-6 text-primary" />
            </div>
            <div>
              <h1 className="text-xl font-semibold tracking-tight">Troubleshooting</h1>
              <p className="text-sm text-muted-foreground">Diagnose and fix addon problems</p>
            </div>
          </div>
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-auto">
        <div className="container mx-auto max-w-2xl space-y-8 px-4 py-8">
          <Section title="Diagnostics" description="Errors found in the game's addon log">
            <DiagnosticsPanel
              data={diagnosticData}
              error={diagnosticsError}
              hasResult={hasDiagnostics}
              isLoading={isLoadingDiagnostics}
              onRescan={runDiagnostics}
            />
          </Section>

          <Section
            title="Addon recovery"
            description="When every addon stops working, the addon_settings file is usually corrupted"
          >
            <AddonRecovery />
          </Section>

          <Section
            title="Problem with one addon"
            description="Report it to the addon author from the Dashboard"
          >
            <ReportIssueGuide />
          </Section>
        </div>
      </main>
    </div>
  )
}
