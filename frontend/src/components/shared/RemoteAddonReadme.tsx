import { useState, type ComponentProps } from 'react';
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPrimitive
} from '@/components/ui/dialog'
import Markdown from 'react-markdown'
import { XIcon } from 'lucide-react';

interface Props {
  readme: string
}

export const RemoteAddonReadme = ({ readme }: Props) => {
  const [selectedImage, setSelectedImage] = useState<HTMLImageElement | null>(null);

  const components = {
      a: ({ node, ...props }: ComponentProps<"a"> & { node?: unknown }) => (
        <a
          className="text-blue-400 hover:text-blue-500 hover:underline transition-all"
          {...props}
        />
      ),
      h1: ({ node, ...props }: ComponentProps<"h1"> & { node?: unknown }) => (
        <h1
          className="my-2 border-b border-gray-600 pb-2 text-3xl font-semibold"
          {...props}
        />
      ),
      h2: ({ node, ...props }: ComponentProps<"h2"> & { node?: unknown }) => (
        <h2
          className="my-2 border-b border-gray-600 pb-2 text-2xl font-semibold"
          {...props}
        />
      ),
      h3: ({ node, ...props }: ComponentProps<"h3"> & { node?: unknown }) => (
        <h3
          className="my-2 border-b border-gray-600 pb-2 text-xl font-semibold"
          {...props}
        />
      ),
      h4: ({ node, ...props }: ComponentProps<"h4"> & { node?: unknown }) => (
        <h4
          className="my-2 border-b border-gray-600 pb-2 text-lg font-semibold"
          {...props}
        />
      ),
      h5: ({ node, ...props }: ComponentProps<"h5"> & { node?: unknown }) => (
        <h5
          className="my-2 border-b border-gray-600 pb-2 text-base font-semibold"
          {...props}
        />
      ),
      h6: ({ node, ...props }: ComponentProps<"h6"> & { node?: unknown }) => (
        <h6
          className="my-2 border-b border-gray-600 pb-2 text-sm font-bold"
          {...props}
        />
      ),
      img: ({ node, ...props }: ComponentProps<"img"> & { node?: unknown }) => (
        <img
        onClick={(event) => setSelectedImage(event.currentTarget)}
          className="my-3 cursor-pointer transition-all scale-[90%] hover:scale-[95%]"
          style={{ transitionDuration: "400ms" }}
          {...props}
        />
      ),
      code: ({
        node,
        inline,
        className,
        children,
        ...props
      }: ComponentProps<"code"> & { node?: unknown; inline?: boolean }) => {
        const match = /language-(\w+)/.exec(className || "");
        return !inline && match ? (
          <code className={className} {...props}>
            {children}
          </code>
        ) : (
          <code
            className="bg-muted py-0.5 px-1 overflow-x-auto rounded-sm font-mono"
            {...props}
          >
            {children}
          </code>
        );
      },
      pre: ({ node, ...props }: ComponentProps<"pre"> & { node?: unknown }) => (
        <pre
          className="bg-muted p-4 my-4 overflow-x-auto rounded-md font-mono text-muted-foreground"
          {...props}
        />
      ),
      p: ({ node, ...props }: ComponentProps<"p"> & { node?: unknown }) => (
        <p className="my-2" {...props} />
      ),
      ul: ({ node, ...props }: ComponentProps<"ul"> & { node?: unknown }) => (
        <ul className="my-5 list-disc list-inside" {...props} />
      ),
      ol: ({ node, ...props }: ComponentProps<"ol"> & { node?: unknown }) => (
        <ol className="my-5 list-decimal list-inside" {...props} />
      ),
    };

  return (
    <>
      {selectedImage && (
        <Dialog open={selectedImage !== null} onOpenChange={(open: boolean) => {
          if(!open) {
            setSelectedImage(null)
          }
        }}>
          <DialogOverlay className='fixed inset-0 z-50 bg-black/80'>
            <div className='content-wrapper z-50'>
              <DialogPrimitive.Content className='!p-0 !gap-0 !bg-transparent !border-0 !shadow-none !w-auto !h-auto'>
                  <div className='relative w-[95vw] h-[95vh] flex items-center justify-center'>
                    <div className='relative flex items-center justify-center min-w-[60vw] min-h-[60vh]'>
                      <div className='relative inline-block'>
                        <img 
                          src={selectedImage.src} 
                          alt={selectedImage.alt} 
                          className='rounded-lg max-w-[95vw] max-h-[95vh] w-auto h-auto object-contain' 
                        />
                        <DialogClose className='absolute -top-4 -right-4 rounded-full bg-background p-2 text-foreground hover:bg-muted transition-colors shadow-lg'>
                          <XIcon className='size-4' />
                          <span className='sr-only'>Close</span>
                        </DialogClose>
                      </div>
                    </div>
                  </div>
              </DialogPrimitive.Content>
            </div>
          </DialogOverlay>
        </Dialog>
      )}
      <Markdown components={components}>{readme}</Markdown>
    </>
  )
}
