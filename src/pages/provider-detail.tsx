import { ProviderCard } from "@/components/provider-card"
import { MachineBadge } from "@/components/machine-badge"
import type { PluginDisplayState } from "@/lib/plugin-types"
import type { DisplayMode, ResetTimerDisplayMode } from "@/lib/settings"
import { useAppSyncStore } from "@/stores/app-sync-store"
import { combineMetricLines } from "@/lib/aggregate-metrics"

interface ProviderDetailPageProps {
  plugin: PluginDisplayState | null
  onRetry?: () => void
  displayMode: DisplayMode
  resetTimerDisplayMode: ResetTimerDisplayMode
  onResetTimerDisplayModeToggle?: () => void
}

export function ProviderDetailPage({
  plugin,
  onRetry,
  displayMode,
  resetTimerDisplayMode,
  onResetTimerDisplayModeToggle,
}: ProviderDetailPageProps) {
  const { syncEnabled, remoteMachines } = useAppSyncStore()

  if (!plugin) {
    return (
      <div className="text-center text-muted-foreground py-8">
        Provider not found
      </div>
    )
  }

  // Find this provider's snapshots on remote machines
  const remoteSources = syncEnabled
    ? remoteMachines
        .map((m) => {
          const snap = m.snapshots.find((s) => s.providerId === plugin.meta.id)
          return snap ? { machine: m, snapshot: snap } : null
        })
        .filter((x): x is NonNullable<typeof x> => x !== null)
    : []

  const showCombined = remoteSources.length > 0 && plugin.data
  const localLines = plugin.data?.lines ?? []
  const combinedLines = showCombined
    ? combineMetricLines([
        { machine: { id: "__local__", name: "This machine", isLocal: true }, lines: localLines },
        ...remoteSources.map((r) => ({
          machine: { id: r.machine.machineId, name: r.machine.machineName, isLocal: false },
          lines: r.snapshot.lines,
        })),
      ])
    : localLines

  return (
    <div className="space-y-3">
      <ProviderCard
        name={plugin.meta.name}
        plan={plugin.data?.plan}
        links={plugin.meta.links}
        showSeparator={false}
        loading={plugin.loading}
        error={plugin.error}
        lines={combinedLines}
        skeletonLines={plugin.meta.lines}
        lastManualRefreshAt={plugin.lastManualRefreshAt}
        onRetry={onRetry}
        scopeFilter="all"
        displayMode={displayMode}
        resetTimerDisplayMode={resetTimerDisplayMode}
        onResetTimerDisplayModeToggle={onResetTimerDisplayModeToggle}
      />

      {showCombined && (
        <div className="px-3 pb-2">
          <div className="text-[10px] text-muted-foreground mb-1">
            Combined from {remoteSources.length + 1} machines:
          </div>
          <div className="flex flex-wrap gap-1">
            <MachineBadge name="This machine" isLocal />
            {remoteSources.map((r) => (
              <MachineBadge
                key={r.machine.machineId}
                name={r.machine.machineName}
                lastSeenAt={r.machine.lastSeenAt}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
