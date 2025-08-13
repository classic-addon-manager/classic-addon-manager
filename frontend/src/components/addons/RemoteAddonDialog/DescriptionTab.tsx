import { RemoteAddonReadme } from '@/components/shared/RemoteAddonReadme.tsx'

interface DescriptionTabProps {
  readme: string
}

export const DescriptionTab = ({ readme }: DescriptionTabProps) => {
  return (
    <div className="max-w-none">
      <RemoteAddonReadme readme={readme} />
    </div>
  )
}
