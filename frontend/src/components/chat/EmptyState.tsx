import supportDaru from '@/assets/images/support_daru.webp'

export const EmptyState = () => {
  return (
    <div className="h-full flex flex-col items-center justify-center space-y-6 text-center px-4">
      <div className="space-y-6 max-h-full">
        <img
          src={supportDaru}
          alt="Support Daru"
          className="w-[35vh] h-[35vh] max-w-[280px] max-h-[280px] min-w-[160px] min-h-[160px] object-contain mx-auto drop-shadow-lg hover:scale-105 transition-transform duration-300"
        />
        <div>
          <h3 className="text-xl font-semibold text-primary mb-2">Welcome to Daru's Help Desk!</h3>
          <p className="text-sm text-muted-foreground max-w-md mx-auto leading-relaxed">
            I'm your friendly Daru assistant, ready to help with all your addon needs. Feel free to
            ask me anything!
          </p>
        </div>
      </div>
    </div>
  )
}
