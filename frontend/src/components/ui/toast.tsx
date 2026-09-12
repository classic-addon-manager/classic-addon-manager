import { toast as sonnerToast } from 'sonner'

import { Toast, type ToastProps } from '@/components/ui/toast-content'

export function toast(toast: Omit<ToastProps, 'id'>) {
  return sonnerToast.custom(
    id => (
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
      dismissible: true,
      style: {
        zIndex: 9999,
      },
    }
  )
}
