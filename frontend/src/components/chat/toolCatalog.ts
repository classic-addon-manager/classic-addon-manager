import {
  BookTextIcon,
  FileTextIcon,
  InfoIcon,
  type LucideIcon,
  PackageSearchIcon,
  ScrollTextIcon,
  SearchIcon,
  TagIcon,
  WrenchIcon,
} from 'lucide-react'

export interface ToolMeta {
  /** Shown while the call is still in flight. */
  running: string
  /** Shown once the turn has moved past this call. */
  done: string
  icon: LucideIcon
}

const TOOL_CATALOG: Record<string, ToolMeta> = {
  list_addons: {
    running: 'Getting addons...',
    done: 'Listed addons',
    icon: PackageSearchIcon,
  },
  get_latest_release: {
    running: 'Checking the latest release...',
    done: 'Checked the latest release',
    icon: TagIcon,
  },
  get_addon_readme: {
    running: 'Opening the addon README...',
    done: 'Read the addon README',
    icon: FileTextIcon,
  },
  get_addon_manager_documentation: {
    running: 'Fetching manager instructions...',
    done: 'Read the manager instructions',
    icon: BookTextIcon,
  },
  get_addon_details: {
    running: 'Gathering addon details...',
    done: 'Gathered addon details',
    icon: InfoIcon,
  },
  search_archeage_wiki: {
    running: 'Searching the ArcheAge Classic wiki...',
    done: 'Searched the ArcheAge Classic wiki',
    icon: SearchIcon,
  },
  get_archeage_wiki_page_content: {
    running: 'Reading the wiki page...',
    done: 'Read a wiki page',
    icon: ScrollTextIcon,
  },
}

const UNKNOWN_TOOL: ToolMeta = {
  running: 'Working on something...',
  done: 'Used a tool',
  icon: WrenchIcon,
}

export const getToolMeta = (action: string): ToolMeta => TOOL_CATALOG[action] ?? UNKNOWN_TOOL
