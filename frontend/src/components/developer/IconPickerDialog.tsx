import { ImagePlusIcon, Trash2 } from 'lucide-react'
import { useCallback, useRef, useState } from 'react'

import { validateIconFile } from '@/components/developer/validate'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { cn } from '@/lib/utils'

function BlobImg({ file, className }: { file: File; className?: string }) {
  const setRef = useCallback(
    (node: HTMLImageElement | null) => {
      if (!node) return
      const url = URL.createObjectURL(file)
      node.src = url
      return () => URL.revokeObjectURL(url)
    },
    [file]
  )
  return <img alt="" className={className} ref={setRef} />
}

function IconPreview({
  file,
  url,
  className,
}: {
  file: File | null
  url: string | null
  className?: string
}) {
  if (file === null && url === null) {
    return (
      <div
        className={cn(
          'flex items-center justify-center rounded-lg border bg-card text-primary',
          className
        )}
      >
        <ImagePlusIcon className="size-1/2" />
      </div>
    )
  }
  if (file !== null) {
    return <BlobImg file={file} className={cn('rounded-lg border object-cover', className)} />
  }
  return (
    <img
      src={url ?? undefined}
      alt=""
      className={cn('rounded-lg border object-cover', className)}
    />
  )
}

export function IconPickerDialog({
  open,
  onOpenChange,
  currentIconUrl,
  onSave,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentIconUrl: string | null
  /** Commits the staged pick, null removes the icon. */
  onSave: (icon: File | null) => Promise<string | null>
}) {
  const [saving, setSaving] = useState(false)

  return (
    <Dialog
      open={open}
      onOpenChange={nextOpen => {
        if (nextOpen || !saving) onOpenChange(nextOpen)
      }}
    >
      <DialogContent className="sm:max-w-xl" showCloseButton={!saving}>
        <IconPickerForm
          currentIconUrl={currentIconUrl}
          onSave={onSave}
          onSaved={() => onOpenChange(false)}
          onCancel={() => onOpenChange(false)}
          saving={saving}
          setSaving={setSaving}
        />
      </DialogContent>
    </Dialog>
  )
}

function IconPickerForm({
  currentIconUrl,
  onSave,
  onSaved,
  onCancel,
  saving,
  setSaving,
}: {
  currentIconUrl: string | null
  onSave: (icon: File | null) => Promise<string | null>
  onSaved: () => void
  onCancel: () => void
  saving: boolean
  setSaving: (saving: boolean) => void
}) {
  const [staged, setStaged] = useState<File | null>(null)
  const [remove, setRemove] = useState(false)
  const [errors, setErrors] = useState<string[]>([])
  const [dragOver, setDragOver] = useState(false)
  const [validating, setValidating] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const sessionRef = useRef(0)

  const busy = validating || saving
  const hasPendingChange = staged !== null || remove
  const previewFile = remove ? null : staged

  const stage = async (file: File | undefined) => {
    if (!file || busy) return
    const session = ++sessionRef.current
    setValidating(true)
    setStaged(null)
    setRemove(false)
    setErrors([])
    const problems = await validateIconFile(file)
    if (session !== sessionRef.current) return
    setValidating(false)
    if (problems.length > 0) {
      setErrors(problems)
      return
    }
    setStaged(file)
  }

  const handleSave = async () => {
    if (!hasPendingChange || busy) return
    const selected = remove ? null : staged
    setSaving(true)
    const error = await onSave(selected)
    setSaving(false)
    if (error) {
      setErrors([error])
      return
    }
    onSaved()
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>Addon icon</DialogTitle>
        <DialogDescription>
          Shown in search results, the detail header, and installed lists.
        </DialogDescription>
      </DialogHeader>

      <div className="grid grid-cols-[1fr_10.5rem] gap-4">
        <div
          role="button"
          tabIndex={busy ? -1 : 0}
          data-file-drop-target
          aria-label="Choose an icon image"
          aria-disabled={busy}
          onClick={() => {
            if (!busy) inputRef.current?.click()
          }}
          onKeyDown={event => {
            if (!busy && (event.key === 'Enter' || event.key === ' ')) {
              event.preventDefault()
              inputRef.current?.click()
            }
          }}
          onDragOver={event => {
            event.preventDefault()
            if (!busy) setDragOver(true)
          }}
          onDragLeave={() => setDragOver(false)}
          onDrop={event => {
            event.preventDefault()
            setDragOver(false)
            if (!busy) void stage(event.dataTransfer.files[0])
          }}
          className={cn(
            'flex min-h-40 cursor-pointer flex-col items-center justify-center gap-2 rounded-lg border-[1.5px] border-dashed p-4 text-center transition-colors hover:border-primary hover:bg-primary/5',
            dragOver && 'border-primary bg-primary/10',
            errors.length > 0 && 'border-destructive'
          )}
        >
          <ImagePlusIcon className="size-5 text-primary" />
          <p className="text-sm font-semibold">Drop image or click to browse</p>
          <p className="text-xs text-muted-foreground">Square PNG · min 50×50 · max 3 MB</p>
        </div>
        <input
          ref={inputRef}
          type="file"
          accept="image/png"
          disabled={busy}
          className="hidden"
          onChange={event => {
            void stage(event.target.files?.[0])
            event.target.value = ''
          }}
        />

        <div className="flex flex-col items-center gap-2.5 rounded-lg border p-3">
          <span className="text-[10px] uppercase tracking-wide text-muted-foreground">Preview</span>
          <div className="flex items-end gap-3">
            <IconPreview
              file={previewFile}
              url={remove ? null : currentIconUrl}
              className="size-24"
            />
            <IconPreview
              file={previewFile}
              url={remove ? null : currentIconUrl}
              className="size-10"
            />
          </div>
          <span className="text-center text-[10px] text-muted-foreground">
            96px · 40px preview sizes
          </span>
          <span className="text-center text-[11px] text-muted-foreground">
            {staged
              ? `${staged.name} · ${Math.ceil(staged.size / 1024)} KB`
              : remove
                ? 'Icon will be removed'
                : currentIconUrl
                  ? 'Current icon'
                  : 'No image selected'}
          </span>
        </div>
      </div>

      {errors.length > 0 && (
        <div className="space-y-1">
          {errors.map((message, index) => (
            <p key={`${message}-${index}`} className="text-xs font-medium text-destructive">
              {message}
            </p>
          ))}
        </div>
      )}

      <DialogFooter className="gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={busy || (!hasPendingChange && currentIconUrl === null)}
          onClick={() => {
            sessionRef.current += 1
            setStaged(null)
            setRemove(true)
            setErrors([])
          }}
        >
          <Trash2 />
          Remove
        </Button>
        <div className="flex gap-2">
          <Button type="button" variant="outline" size="sm" disabled={saving} onClick={onCancel}>
            Cancel
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={!hasPendingChange || busy}
            onClick={() => void handleSave()}
          >
            {saving ? 'Uploading…' : 'Save'}
          </Button>
        </div>
      </DialogFooter>
    </>
  )
}
