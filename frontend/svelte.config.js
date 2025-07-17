import { sveltePreprocess } from 'svelte-preprocess'

export default {
  // Consult https://github.com/sveltejs/svelte-preprocess
  // for more information about preprocessors
  preprocess: sveltePreprocess(),
  compilerOptions: {
    // disable all warnings coming from node_modules and all accessibility warnings
    warningFilter: (warning) => !warning.filename?.includes('node_modules') && !warning.code.startsWith('a11y')
  }
}
