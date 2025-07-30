import type { WritableAtom } from 'jotai/index'
import { createContext } from 'react'

export const UpdateDialogAtomContext = createContext<WritableAtom<boolean, any, any> | null>(null)
