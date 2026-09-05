import { Browser } from '@wailsio/runtime'
import { GithubIcon, GitPullRequest } from 'lucide-react'

import type { OwnedSubmission } from '@/components/developer/ownedParse'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { formatToLocalDate } from '@/lib/utils'

export const SubmissionRow = ({ submission }: { submission: OwnedSubmission }) => {
  const showPr = submission.htmlUrl.startsWith('https://github.com/')
  const showRepo = submission.repo !== null

  return (
    <div className="flex items-center gap-3 rounded-xl border border-border/50 bg-muted/30 px-4 py-3">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate font-medium">{submission.title}</span>
          {submission.status === 'open' ? (
            <Badge>In review</Badge>
          ) : (
            <Badge variant="destructive">Rejected</Badge>
          )}
        </div>
        {submission.createdAt !== null && (
          <p className="text-xs text-muted-foreground">{formatToLocalDate(submission.createdAt)}</p>
        )}
      </div>
      <div className="ml-auto flex">
        {showPr && (
          <TooltipProvider>
            <Tooltip delayDuration={100}>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 text-muted-foreground hover:text-primary"
                  onClick={() => void Browser.OpenURL(submission.htmlUrl)}
                  aria-label="View catalog pull request"
                >
                  <GitPullRequest className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>View PR</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
        {showRepo && (
          <TooltipProvider>
            <Tooltip delayDuration={100}>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 text-muted-foreground hover:text-primary"
                  onClick={() => void Browser.OpenURL(`https://github.com/${submission.repo}`)}
                  aria-label="View code on GitHub"
                >
                  <GithubIcon className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>View code</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>
    </div>
  )
}
