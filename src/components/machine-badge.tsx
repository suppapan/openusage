import { cn } from "@/lib/utils"

function relativeTime(isoDate: string): string {
  const now = Date.now()
  const then = new Date(isoDate).getTime()
  const diffMs = now - then

  if (diffMs < 0) return "just now"

  const seconds = Math.floor(diffMs / 1000)
  if (seconds < 60) return "just now"

  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`

  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ago`

  const days = Math.floor(hours / 24)
  return `${days}d ago`
}

type MachineBadgeProps = {
  name: string
  lastSeenAt?: string
  isLocal?: boolean
  className?: string
}

export function MachineBadge({ name, lastSeenAt, isLocal, className }: MachineBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium",
        isLocal
          ? "bg-primary/10 text-primary"
          : "bg-muted text-muted-foreground",
        className
      )}
    >
      <span className={cn(
        "inline-block w-1.5 h-1.5 rounded-full",
        isLocal ? "bg-primary" : "bg-muted-foreground/50"
      )} />
      {isLocal ? "This machine" : name}
      {lastSeenAt && !isLocal && (
        <span className="text-muted-foreground/70">{relativeTime(lastSeenAt)}</span>
      )}
    </span>
  )
}
