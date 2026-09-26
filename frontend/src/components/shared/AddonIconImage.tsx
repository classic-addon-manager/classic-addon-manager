import { BlocksIcon } from 'lucide-react'
import { useState } from 'react'

interface AddonIconImageProps {
  src: string | null
  alt: string
  imageClassName: string
  fallbackClassName: string
  fallbackIconClassName: string
}

export const AddonIconImage = ({
  src,
  alt,
  imageClassName,
  fallbackClassName,
  fallbackIconClassName,
}: AddonIconImageProps) => {
  // Remembering the failed URL instead of a flag lets a new URL retry without an effect.
  const [failedSrc, setFailedSrc] = useState<string | null>(null)

  if (src !== null && src !== failedSrc) {
    return (
      <img
        className={imageClassName}
        src={src}
        alt={alt}
        loading="lazy"
        onError={() => setFailedSrc(src)}
      />
    )
  }

  return (
    <div className={fallbackClassName}>
      <BlocksIcon className={fallbackIconClassName} />
    </div>
  )
}
