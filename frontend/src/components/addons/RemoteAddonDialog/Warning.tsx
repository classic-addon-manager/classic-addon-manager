import { AlertTriangleIcon } from 'lucide-react'

interface WarningProps {
  text: string | null | undefined
}

export const Warning = ({ text }: WarningProps) => {
  if (!text) return null

  return (
    <div className="flex items-start gap-2 border-y border-transparent bg-orange-300 px-6 py-3 text-sm text-black shadow">
      <AlertTriangleIcon className="mt-0.5 h-4 w-4 shrink-0" />
      <p>{text}</p>
    </div>
  )
}
