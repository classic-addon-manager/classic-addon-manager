import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'

export function AlreadyOpenDialog({
  open,
  onDismiss,
  onResume,
}: {
  open: boolean
  onDismiss: () => void
  onResume: () => void
}) {
  return (
    <AlertDialog
      open={open}
      onOpenChange={next => {
        if (!next) onDismiss()
      }}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Submission already exists</AlertDialogTitle>
          <AlertDialogDescription>
            You already have an open or rejected submission for this name.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={onDismiss}>Keep editing</AlertDialogCancel>
          <AlertDialogAction onClick={onResume}>Resume existing</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
