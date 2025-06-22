import { toast as sonnerToast } from 'sonner'
import { X } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

interface ToastProps {
  id: string | number
  title: string
  description: string
  icon?: LucideIcon
  button?: {
    label: string
    onClick: () => void
  }
}

export function toast(toast: Omit<ToastProps, 'id'>) {
  return sonnerToast.custom(
    (id) => (
      <Toast
        id={id}
        title={toast.title}
        description={toast.description}
        button={toast.button}
        icon={toast.icon}
      />
    ),
    {
      duration: 4000,
      dismissible: true
    }
  )
}

function Toast(props: ToastProps) {
  const { title, description, button, icon: Icon, id } = props

  return (
    <div className="group relative flex w-full items-center overflow-hidden rounded-lg border bg-background p-4 shadow-lg md:w-[364px]">
      <button
        onClick={() => sonnerToast.dismiss(id)}
        className="absolute right-2 top-2 rounded-md p-1 text-foreground/50 transition-opacity hover:text-foreground"
      >
        <X className="h-4 w-4" />
      </button>
      <div className="flex items-center gap-3">
        {Icon && <Icon className="h-5 w-5 shrink-0 text-foreground" />}
        <div className={`min-w-0 flex-1 ${button ? 'mr-4' : ''}`}>
          <p className="font-semibold text-foreground truncate">{title}</p>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
      </div>
      {button && (
        <button
          onClick={() => {
            button.onClick()
            sonnerToast.dismiss(id)
          }}
          className="inline-flex h-8 shrink-0 items-center justify-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground ring-offset-background transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
        >
          {button.label}
        </button>
      )}
    </div>
  )
}

