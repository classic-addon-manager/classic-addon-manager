import { Grid } from './RemoteAddon/Grid'
import { List } from './RemoteAddon/List'
import type { RemoteAddonItemProps } from './RemoteAddon/types'
import type { AddonViewMode } from './types'

interface RemoteAddonProps extends RemoteAddonItemProps {
  variant?: AddonViewMode
}

export const RemoteAddon = ({ manifest, installed, variant = 'list' }: RemoteAddonProps) => {
  if (variant === 'grid') {
    return <Grid manifest={manifest} installed={installed} />
  }

  return <List manifest={manifest} installed={installed} />
}
