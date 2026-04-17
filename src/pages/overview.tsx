import { useState } from "react"
import { ProviderCard } from "@/components/provider-card"
import { MachineBadge } from "@/components/machine-badge"
import type { PluginDisplayState } from "@/lib/plugin-types"
import type { DisplayMode, ResetTimerDisplayMode } from "@/lib/settings"
import { useAppSyncStore } from "@/stores/app-sync-store"

interface OverviewPageProps {
  plugins: PluginDisplayState[]
  onRetryPlugin?: (pluginId: string) => void
  displayMode: DisplayMode
  resetTimerDisplayMode: ResetTimerDisplayMode
  onResetTimerDisplayModeToggle?: () => void
}

export function OverviewPage({
  plugins,
  onRetryPlugin,
  displayMode,
  resetTimerDisplayMode,
  onResetTimerDisplayModeToggle,
}: OverviewPageProps) {
  const { syncEnabled, remoteMachines } = useAppSyncStore()
  const [viewMode, setViewMode] = useState<"local" | "all">("local")

  const hasRemoteMachines = syncEnabled && remoteMachines.length > 0

  if (plugins.length === 0 && !hasRemoteMachines) {
    return (
      <div className="text-center text-muted-foreground py-8">
        No providers enabled
      </div>
    )
  }

  return (
    <div>
      {hasRemoteMachines && (
        <div className="flex gap-1 mb-2 px-0.5">
          <button
            onClick={() => setViewMode("local")}
            className={`text-xs px-2 py-0.5 rounded-full transition-colors ${
              viewMode === "local"
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:bg-muted/80"
            }`}
          >
            This Machine
          </button>
          <button
            onClick={() => setViewMode("all")}
            className={`text-xs px-2 py-0.5 rounded-full transition-colors ${
              viewMode === "all"
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:bg-muted/80"
            }`}
          >
            All Machines ({remoteMachines.length + 1})
          </button>
        </div>
      )}

      {/* Local machine plugins */}
      {plugins.map((plugin, index) => (
        <div key={plugin.meta.id}>
          {viewMode === "all" && hasRemoteMachines && index === 0 && (
            <div className="px-0.5 mb-1">
              <MachineBadge name="" isLocal />
            </div>
          )}
          <ProviderCard
            name={plugin.meta.name}
            plan={plugin.data?.plan}
            showSeparator={viewMode === "local" ? index < plugins.length - 1 : true}
            loading={plugin.loading}
            error={plugin.error}
            lines={plugin.data?.lines ?? []}
            skeletonLines={plugin.meta.lines}
            lastManualRefreshAt={plugin.lastManualRefreshAt}
            onRetry={onRetryPlugin ? () => onRetryPlugin(plugin.meta.id) : undefined}
            scopeFilter="overview"
            displayMode={displayMode}
            resetTimerDisplayMode={resetTimerDisplayMode}
            onResetTimerDisplayModeToggle={onResetTimerDisplayModeToggle}
          />
        </div>
      ))}

      {/* Remote machine snapshots */}
      {viewMode === "all" && hasRemoteMachines && remoteMachines.map((machine) => (
        <div key={machine.machineId}>
          <div className="px-0.5 mt-2 mb-1">
            <MachineBadge
              name={machine.machineName}
              lastSeenAt={machine.lastSeenAt}
            />
          </div>
          {machine.snapshots.map((snapshot, sIdx) => (
            <ProviderCard
              key={`${machine.machineId}-${snapshot.providerId}`}
              name={snapshot.displayName}
              plan={snapshot.plan}
              showSeparator={sIdx < machine.snapshots.length - 1}
              loading={false}
              error={null}
              lines={snapshot.lines}
              skeletonLines={[]}
              lastManualRefreshAt={null}
              scopeFilter="overview"
              displayMode={displayMode}
              resetTimerDisplayMode={resetTimerDisplayMode}
              onResetTimerDisplayModeToggle={onResetTimerDisplayModeToggle}
            />
          ))}
        </div>
      ))}
    </div>
  )
}
