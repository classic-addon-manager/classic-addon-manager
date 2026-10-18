import { AlertTriangleIcon, CheckIcon } from 'lucide-react'
import { useState } from 'react'

import { deprecateAddon } from '@/components/developer/declarationApi.ts'
import type { OwnedAddon } from '@/components/developer/ownedParse'
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
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast'
import { useAddonCatalog } from '@/lib/catalog'

const REASON_MAX_LENGTH = 500
// Radix Select items cannot use an empty value, so "no replacement" needs its own.
const NO_REPLACEMENT = '__none__'

export function DeprecateAddonDialog({
  addon,
  ownedNames,
  open,
  onOpenChange,
  onDeprecated,
}: {
  addon: OwnedAddon
  /** Names of every addon this developer owns, their dependents never need review. */
  ownedNames: ReadonlySet<string>
  open: boolean
  onOpenChange: (open: boolean) => void
  onDeprecated: () => void
}) {
  const { data: catalog = [] } = useAddonCatalog()
  const [reason, setReason] = useState('')
  const [replacement, setReplacement] = useState(NO_REPLACEMENT)
  const [busy, setBusy] = useState(false)

  const replacementOptions = catalog
    .filter(manifest => manifest.name !== addon.name && !manifest.deprecated)
    .sort((a, b) => a.alias.localeCompare(b.alias))
  // A preview only: the API decides whether review is needed when the request arrives.
  const outsideDependents = catalog.filter(
    manifest =>
      !ownedNames.has(manifest.name) &&
      !manifest.deprecated &&
      manifest.dependencies.includes(addon.name)
  )
  const trimmedReason = reason.trim()

  const handleOpenChange = (next: boolean) => {
    if (!busy) onOpenChange(next)
  }

  const handleSubmit = async () => {
    if (trimmedReason === '' || busy) return
    setBusy(true)
    const result = await deprecateAddon(addon.uuid, {
      reason: trimmedReason,
      replacement: replacement === NO_REPLACEMENT ? null : replacement,
    })
    setBusy(false)

    if (result.status === 'deprecated' || result.status === 'review') {
      toast({
        title: result.status === 'deprecated' ? 'Addon deprecated' : 'Deprecation sent for review',
        description:
          result.status === 'deprecated'
            ? `Users who have ${addon.alias} installed are now asked to uninstall it.`
            : 'Addons by other developers depend on it, so a reviewer will confirm first.',
        icon: CheckIcon,
      })
      onOpenChange(false)
      onDeprecated()
      return
    }
    toast({ title: 'Deprecate addon', description: result.message, icon: AlertTriangleIcon })
  }

  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Deprecate {addon.alias}?</AlertDialogTitle>
          <AlertDialogDescription>
            It is hidden from browsing, cannot be installed or updated, and everyone who has it
            installed is asked to uninstall it. You can restore it later.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="deprecate-reason">Reason</Label>
            <Textarea
              id="deprecate-reason"
              value={reason}
              maxLength={REASON_MAX_LENGTH}
              disabled={busy}
              placeholder="Shown to users, e.g. no longer maintained, merged into another addon"
              onChange={event => setReason(event.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="deprecate-replacement">Replacement (optional)</Label>
            <Select value={replacement} onValueChange={setReplacement} disabled={busy}>
              <SelectTrigger id="deprecate-replacement" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NO_REPLACEMENT}>No replacement</SelectItem>
                {replacementOptions.map(manifest => (
                  <SelectItem key={manifest.name} value={manifest.name}>
                    {manifest.alias}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {outsideDependents.length > 0 && (
            <p className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 text-sm">
              {outsideDependents.map(manifest => manifest.alias).join(', ')}{' '}
              {outsideDependents.length === 1 ? 'depends' : 'depend'} on this addon and{' '}
              {outsideDependents.length === 1 ? 'is' : 'are'} published by another developer, so
              this deprecation goes to review before it takes effect.
            </p>
          )}
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={busy}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            disabled={busy || trimmedReason === ''}
            onClick={event => {
              event.preventDefault()
              void handleSubmit()
            }}
          >
            {busy
              ? 'Deprecating...'
              : outsideDependents.length > 0
                ? 'Submit for review'
                : 'Deprecate addon'}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
