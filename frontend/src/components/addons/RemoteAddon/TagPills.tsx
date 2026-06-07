interface TagPillsProps {
  tags: string[]
  maxTags?: number
  className?: string
}

export const TagPills = ({ tags, maxTags, className }: TagPillsProps) => {
  if (tags.length === 0) return null

  const visibleTags = maxTags === undefined ? tags : tags.slice(0, maxTags)
  const hiddenCount = maxTags === undefined ? 0 : Math.max(0, tags.length - maxTags)

  return (
    <div className={`flex items-center gap-1 flex-wrap ${className ?? ''}`}>
      {visibleTags.map((tag, index) => (
        <span
          key={`${tag}-${index}`}
          className="text-[10px] font-medium px-1.5 py-0.5 rounded-md bg-secondary/50 border border-secondary/50 text-secondary-foreground shadow-xs whitespace-nowrap transition-all"
        >
          {tag}
        </span>
      ))}
      {hiddenCount > 0 && (
        <span className="text-[10px] text-muted-foreground">+{hiddenCount} more</span>
      )}
    </div>
  )
}
