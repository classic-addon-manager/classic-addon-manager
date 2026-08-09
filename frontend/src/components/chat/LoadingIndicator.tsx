export const LoadingIndicator = () => {
  return (
    <div className="flex items-center gap-1.5 py-1">
      <span className="mr-1 text-xs text-muted-foreground/70">Daru is thinking</span>
      <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-primary"></span>
      <span
        className="h-1.5 w-1.5 animate-bounce rounded-full bg-primary"
        style={{ animationDelay: '0.1s' }}
      ></span>
      <span
        className="h-1.5 w-1.5 animate-bounce rounded-full bg-primary"
        style={{ animationDelay: '0.2s' }}
      ></span>
    </div>
  )
}
