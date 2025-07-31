import { ChevronsUpDown, WrenchIcon } from 'lucide-react'

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion'
import { Badge } from '@/components/ui/badge.tsx'
import { Button } from '@/components/ui/button.tsx'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible.tsx'
import { ScrollArea } from '@/components/ui/scroll-area.tsx'

interface AccordionItemData {
  id: string
  title: string
}

interface TroubleshootingProps {
  issueCount?: number
  groupedIssues?: Record<string, Array<{ error: string; file: string }>>
  onResetAddonSettings?: () => void
  onUninstallAllAddons?: () => void
}

const Header = () => {
  return (
    <header className="bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60 border-b">
      <div className="container flex h-16 items-center gap-4 px-4">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-accent rounded-lg">
            <WrenchIcon className="h-6 w-6 text-primary" />
          </div>

          <div>
            <h1 className="text-xl font-semibold tracking-tight">Troubleshooting</h1>
            <p className="text-sm text-muted-foreground">
              Select an option below to help resolve your issue
            </p>
          </div>
        </div>
      </div>
    </header>
  )
}

const SpecificAddonIssueContent = () => (
  <div className="px-6 py-4 bg-muted/20">
    <div className="space-y-3">
      <p className="text-muted-foreground">If a specific addon is not working as expected:</p>
      <ol className="list-decimal list-inside space-y-2 text-muted-foreground">
        <li>Go to the Dashboard</li>
        <li>Right-click the problematic addon</li>
        <li>Select "Report issue"</li>
      </ol>
      <div className="bg-muted/30 p-4 rounded-lg mt-4">
        <p className="text-sm italic">Note: A GitHub account is required to report issues.</p>
      </div>
    </div>
  </div>
)

const NoAddonsWorkingContent = ({
  onResetAddonSettings,
  onUninstallAllAddons,
}: Pick<TroubleshootingProps, 'onResetAddonSettings' | 'onUninstallAllAddons'>) => (
  <div className="px-6 py-4 bg-muted/20">
    <div className="space-y-4">
      <p className="text-muted-foreground">
        When all addons stop working, it's typically due to a corrupted addon_settings file.
      </p>

      <div className="bg-muted/30 p-4 rounded-lg space-y-3">
        <div className="flex items-start gap-2">
          <div className="w-2 h-2 mt-2 rounded-full bg-green-400"></div>
          <p className="flex-1">
            <span className="font-medium text-green-400">Recommended:</span> Try resetting your
            addon settings first
          </p>
        </div>
        <div className="flex items-start gap-2">
          <div className="w-2 h-2 mt-2 rounded-full bg-red-400"></div>
          <p className="flex-1">
            <span className="font-medium text-red-400">Last resort:</span> Uninstall all addons and
            reset settings if the first option doesn't work
          </p>
        </div>
      </div>

      <div className="flex gap-3 mt-4">
        <Button className="w-1/2" onClick={onResetAddonSettings}>
          Reset addon settings
        </Button>
        <Button variant="destructive" className="w-1/2" onClick={onUninstallAllAddons}>
          Uninstall all addons
        </Button>
      </div>
    </div>
  </div>
)

const DiagnosticResultsContent = ({
  issueCount = 0,
  groupedIssues = {},
}: Pick<TroubleshootingProps, 'issueCount' | 'groupedIssues'>) => (
  <div className="px-6 py-4 bg-muted/20">
    {issueCount === 0 ? (
      <div className="flex items-center justify-center py-8">
        <p className="text-green-400 font-medium">No issues detected in your system! 🎉</p>
      </div>
    ) : (
      <ScrollArea className="rounded-lg border border-zinc-700">
        {Object.keys(groupedIssues).map(addonName => (
          <Collapsible key={addonName} className="border-b border-zinc-700 last:border-0">
            <CollapsibleTrigger className="w-full">
              <div className="flex items-center justify-between p-4 hover:bg-zinc-900/50 transition-colors">
                <div className="flex items-center gap-4 w-full">
                  <h4 className="font-medium">{addonName === 'x2ui' ? 'Addon API' : addonName}</h4>
                  <Badge variant="destructive" className="ml-auto mr-2">
                    {groupedIssues[addonName].length}{' '}
                    {groupedIssues[addonName].length === 1 ? 'issue' : 'issues'}
                  </Badge>
                </div>
                <div className="transition-transform duration-200 data-[state=open]:rotate-180">
                  <ChevronsUpDown size={20} />
                </div>
              </div>
            </CollapsibleTrigger>
            <CollapsibleContent>
              <div className="space-y-2 p-4 bg-zinc-900/30">
                {groupedIssues[addonName].map((issue, i) => (
                  <div
                    key={i}
                    className="rounded-md border border-zinc-700 px-4 py-3 font-mono text-sm bg-zinc-900/50"
                  >
                    <p className="text-red-400 mb-1">Error: {issue.error}</p>
                    <p className="text-zinc-400">
                      File: <span className="text-indigo-400">{issue.file}</span>
                    </p>
                  </div>
                ))}
              </div>
            </CollapsibleContent>
          </Collapsible>
        ))}
      </ScrollArea>
    )}
  </div>
)

const AccordionItemBadge = ({ itemId, issueCount }: { itemId: string; issueCount: number }) => {
  if (itemId !== 'item-3') return null

  return issueCount > 0 ? (
    <Badge variant="destructive" className="ml-auto mr-4">
      {issueCount} {issueCount === 1 ? 'issue' : 'issues'} found
    </Badge>
  ) : (
    <Badge variant="default" className="ml-auto bg-green-600 hover:bg-green-700">
      No issues detected
    </Badge>
  )
}

const renderAccordionContent = (itemId: string, props: TroubleshootingProps) => {
  switch (itemId) {
    case 'item-1':
      return <SpecificAddonIssueContent />
    case 'item-2':
      return (
        <NoAddonsWorkingContent
          onResetAddonSettings={props.onResetAddonSettings}
          onUninstallAllAddons={props.onUninstallAllAddons}
        />
      )
    case 'item-3':
      return (
        <DiagnosticResultsContent
          issueCount={props.issueCount}
          groupedIssues={props.groupedIssues}
        />
      )
    default:
      return null
  }
}

export const Troubleshooting = (props: TroubleshootingProps = {}) => {
  const {
    issueCount = 0,
    groupedIssues = {},
    onResetAddonSettings = () => console.log('Reset addon settingsl'),
    onUninstallAllAddons = () => console.log('Uninstall all addons'),
  } = props

  const accordionItems: AccordionItemData[] = [
    { id: 'item-1', title: 'Specific Addon Issue' },
    { id: 'item-2', title: 'No Addons Are Working' },
    { id: 'item-3', title: 'Diagnostic Results' },
  ]

  return (
    <div className="flex flex-col h-screen">
      <Header />
      <main className="flex-1 overflow-auto">
        <div className="container px-4 py-6">
          <div className="mx-auto max-w-3xl">
            <Accordion type="single" className="space-y-4">
              {accordionItems.map(item => (
                <AccordionItem
                  key={item.id}
                  value={item.id}
                  className="border rounded-lg overflow-hidden"
                >
                  <AccordionTrigger className="hover:bg-muted/50 transition-colors px-6 w-full [&[data-state=open]]:no-underline hover:no-underline no-underline">
                    <div className="flex items-center gap-2 w-full">
                      <span className="text-lg">{item.title}</span>
                      <AccordionItemBadge itemId={item.id} issueCount={issueCount} />
                    </div>
                  </AccordionTrigger>
                  <AccordionContent>
                    {renderAccordionContent(item.id, {
                      issueCount,
                      groupedIssues,
                      onResetAddonSettings,
                      onUninstallAllAddons,
                    })}
                  </AccordionContent>
                </AccordionItem>
              ))}
            </Accordion>
          </div>
        </div>
      </main>
    </div>
  )
}
