import { RatingButtons } from '@/components/addons/RatingButtons'
import { useAddonRating } from '@/lib/addon'

interface AddonRatingButtonsProps {
  addonName: string
  addonAlias: string
  isManaged: boolean
}

export const AddonRatingButtons = (props: AddonRatingButtonsProps) => (
  <AddonRatingButtonsContent key={props.addonName} {...props} />
)

const AddonRatingButtonsContent = ({
  addonName,
  addonAlias,
  isManaged,
}: AddonRatingButtonsProps) => {
  const { rating, isLoading, isSaving, rateAddon } = useAddonRating(
    addonName,
    addonAlias,
    isManaged
  )

  if (!isManaged) return null

  return (
    <RatingButtons rating={rating} onRate={rateAddon} disabled={isLoading || isSaving} size="sm" />
  )
}
