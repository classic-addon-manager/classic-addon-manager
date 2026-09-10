import { publishFormFromPayload, type PublishFormState } from '@/components/developer/constants'
import type { DeclarationValues } from '@/components/developer/types.ts'

export function formToValues(form: PublishFormState): DeclarationValues {
  return {
    name: form.name,
    alias: form.alias,
    description: form.description,
    author: form.author,
    repo: form.repo,
    branch: form.branch,
    tags: [...form.tags],
    keywords: [...form.keywords],
    dependencies: [...form.dependencies],
    kofi: form.kofi,
  }
}

export function valuesToForm(values: DeclarationValues): PublishFormState {
  const keywords = values.keywords
  return publishFormFromPayload({
    name: typeof values.name === 'string' ? values.name : '',
    alias: typeof values.alias === 'string' ? values.alias : '',
    description: typeof values.description === 'string' ? values.description : '',
    author: typeof values.author === 'string' ? values.author : '',
    repo: typeof values.repo === 'string' ? values.repo : '',
    branch: typeof values.branch === 'string' ? values.branch : '',
    tags: Array.isArray(values.tags) ? values.tags : [],
    keywords: Array.isArray(keywords) ? keywords : [],
    dependencies: Array.isArray(values.dependencies) ? values.dependencies : [],
    kofi: typeof values.kofi === 'string' ? values.kofi : '',
  })
}
