import supportDaruAlt from '@/assets/images/support_daru_alt_sm.webp'

export const LoadingIndicator = () => {
  return (
    <div className="flex items-start gap-3 px-4">
      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 overflow-hidden">
        <img src={supportDaruAlt} alt="Daru Assistant" className="w-full h-full object-cover" />
      </div>
      <div className="inline-block rounded-lg px-3 py-1.5 text-sm bg-muted/50 border">
        <div className="flex gap-1.5 items-center">
          <span className="text-muted-foreground/70 text-xs mr-2">Daru is thinking</span>
          <span className="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"></span>
          <span
            className="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: '0.1s' }}
          ></span>
          <span
            className="w-1.5 h-1.5 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: '0.2s' }}
          ></span>
        </div>
      </div>
    </div>
  )
}
