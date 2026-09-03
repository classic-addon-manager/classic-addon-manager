import { Browser } from '@wailsio/runtime'
import { ArrowLeft, Code2, GithubIcon } from 'lucide-react'
import { type ReactNode, useState } from 'react'

import {
  APPROVED_TAGS,
  INITIAL_PUBLISH_FORM,
  isPublishFormDirty,
  type PublishFormState,
} from '@/components/developer/constants'
import { DependenciesCombobox } from '@/components/developer/DependenciesCombobox'
import { KeywordsInput } from '@/components/developer/KeywordsInput'
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
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast'
import { Toggle } from '@/components/ui/toggle'

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
  children,
}: {
  id?: string
  label: string
  hint?: ReactNode
  children: ReactNode
}) => (
  <div className="space-y-1.5">
    <Label htmlFor={id}>{label}</Label>
    {children}
    {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
  </div>
)

interface PublishAddonFormProps {
  onClose: () => void
}

export const PublishAddonForm = ({ onClose }: PublishAddonFormProps) => {
  const [form, setForm] = useState<PublishFormState>(INITIAL_PUBLISH_FORM)
  const [discardOpen, setDiscardOpen] = useState(false)

  const setField = <K extends keyof PublishFormState>(key: K, value: PublishFormState[K]) => {
    setForm(prev => ({ ...prev, [key]: value }))
  }

  const handleBack = () => {
    if (isPublishFormDirty(form)) {
      setDiscardOpen(true)
      return
    }
    onClose()
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="border-b bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
        <div className="container flex h-16 items-center justify-between gap-4 px-4">
          <div className="flex min-w-0 items-center gap-3">
            <div className="rounded-lg bg-primary/10 p-2">
              <Code2 className="h-6 w-6 text-primary" />
            </div>
            <div className="min-w-0">
              <h1 className="text-xl font-semibold tracking-tight">Publish addon</h1>
              <p className="text-sm text-muted-foreground">Fill in the addon declaration.</p>
            </div>
          </div>
          <Button type="button" variant="outline" className="w-32" onClick={handleBack}>
            <ArrowLeft />
            Back
          </Button>
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-auto">
        <div className="container mx-auto max-w-2xl space-y-8 px-4 py-8">
          <Section title="Identity">
            <div className="space-y-4 px-4 py-4">
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <Field id="addon-name" label="Name" hint="No spaces. Starts with A–Z.">
                  <Input
                    id="addon-name"
                    value={form.name}
                    onChange={event => setField('name', event.target.value)}
                  />
                </Field>
                <Field id="addon-alias" label="Alias">
                  <Input
                    id="addon-alias"
                    value={form.alias}
                    onChange={event => setField('alias', event.target.value)}
                  />
                </Field>
              </div>
              <Field id="addon-description" label="Description">
                <Textarea
                  id="addon-description"
                  value={form.description}
                  onChange={event => setField('description', event.target.value)}
                />
              </Field>
              <Field id="addon-author" label="Author">
                <Input
                  id="addon-author"
                  value={form.author}
                  onChange={event => setField('author', event.target.value)}
                />
              </Field>
            </div>
          </Section>

          <Section title="Repository" description="GitHub user/repo and branch.">
            <div className="space-y-4 px-4 py-4">
              <div className="grid gap-4 sm:grid-cols-[1fr_10rem]">
                <Field id="addon-repo" label="Repository" hint="Format: user/repo">
                  <div className="border-input dark:bg-input/30 focus-within:border-ring focus-within:ring-ring/50 flex h-9 overflow-hidden rounded-md border shadow-xs focus-within:ring-[3px]">
                    <span className="text-muted-foreground flex shrink-0 items-center gap-1.5 border-r border-input px-2.5 text-sm">
                      <GithubIcon className="size-3.5" />
                      github.com/
                    </span>
                    <Input
                      id="addon-repo"
                      value={form.repo}
                      placeholder="user/repo"
                      onChange={event => setField('repo', event.target.value)}
                      className="h-full rounded-none border-0 bg-transparent shadow-none focus-visible:ring-0 dark:bg-transparent"
                    />
                  </div>
                </Field>
                <Field id="addon-branch" label="Branch">
                  <Input
                    id="addon-branch"
                    value={form.branch}
                    placeholder="main"
                    onChange={event => setField('branch', event.target.value)}
                  />
                </Field>
              </div>
            </div>
          </Section>

          <Section title="Catalog">
            <div className="space-y-4 px-4 py-4">
              <Field label="Tags" hint="Pick up to 3.">
                <div className="flex flex-wrap gap-1.5">
                  {APPROVED_TAGS.map(tag => {
                    const selected = form.tags.includes(tag)
                    return (
                      <Toggle
                        key={tag}
                        size="sm"
                        variant="outline"
                        pressed={selected}
                        disabled={!selected && form.tags.length >= 3}
                        onPressedChange={pressed => {
                          setForm(prev => {
                            if (pressed) {
                              if (prev.tags.includes(tag) || prev.tags.length >= 3) return prev
                              return { ...prev, tags: [...prev.tags, tag] }
                            }
                            return { ...prev, tags: prev.tags.filter(item => item !== tag) }
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
                hint="Space-separated. No spaces inside a token."
              >
                <KeywordsInput
                  id="addon-keywords"
                  value={form.keywords}
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
              >
                <DependenciesCombobox
                  selected={form.dependencies}
                  onChange={names => setField('dependencies', names)}
                />
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
              >
                <Input
                  id="addon-kofi"
                  value={form.kofi}
                  onChange={event => setField('kofi', event.target.value)}
                />
              </Field>
            </div>
          </Section>
        </div>
      </main>

      <footer className="flex shrink-0 justify-end border-t bg-background/95 px-4 py-3">
        <Button
          type="button"
          onClick={() =>
            toast({
              title: 'Not available yet',
              description: 'Addon validation is not wired up.',
            })
          }
        >
          Validate
        </Button>
      </footer>

      <AlertDialog open={discardOpen} onOpenChange={setDiscardOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Discard this addon?</AlertDialogTitle>
            <AlertDialogDescription>Your declaration will be lost.</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Stay</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={onClose}>
              Discard
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
