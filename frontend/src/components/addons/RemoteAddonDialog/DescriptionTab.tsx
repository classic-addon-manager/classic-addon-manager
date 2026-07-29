import { Readme } from '@/components/shared/Readme'

interface DescriptionTabProps {
  readme: string
}

export const DescriptionTab = ({ readme }: DescriptionTabProps) => {
  return (
    <div className="max-w-none">
      <Readme readme={readme} />
    </div>
  )
}
