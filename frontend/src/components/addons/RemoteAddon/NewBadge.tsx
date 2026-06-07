interface NewBadgeProps {
  isNew: boolean
}

export const NewBadge = ({ isNew }: NewBadgeProps) => {
  if (!isNew) return null

  return (
    <div className="absolute -top-1.5 -right-1.5 bg-red-500/15 text-red-600 border border-red-600/40 text-[8px] font-black px-1.5 py-0.5 rounded-full shadow-xs backdrop-blur-[2px] z-10">
      NEW
    </div>
  )
}
