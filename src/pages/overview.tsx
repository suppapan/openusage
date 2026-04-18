import { useState } from "react"
import { ProviderCard } from "@/components/provider-card"
import { MachineBadge } from "@/components/machine-badge"
import type { PluginDisplayState } from "@/lib/plugin-types"
import type { DisplayMode, ResetTimerDisplayMode } from "@/lib/settings"
import { useAppSyncStore } from "@/stores/app-sync-store"
import { combineByProvider } from "@/lib/aggregate-metrics"

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

  const combined = hasRemoteMachines ? combineByProvider(plugins, remoteMachines) : []

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

      {/* This Machine view: only local plugins */}
      {viewMode === "local" && plugins.map((plugin, index) => (
        <ProviderCard
          key={plugin.meta.id}
          name={plugin.meta.name}
          plan={plugin.data?.plan}
          showSeparator={index < plugins.length - 1}
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
      ))}

      {/* All Machines view: combined cards with per-machine breakdown */}
      {viewMode === "all" && combined.map((entry, index) => (
        <div key={entry.providerId}>
          <ProviderCard
            name={entry.displayName}
            plan={entry.plan}
            showSeparator={index < combined.length - 1}
            loading={false}
            error={null}
            lines={entry.combinedLines}
            skeletonLines={[]}
            lastManualRefreshAt={null}
            scopeFilter="overview"
            displayMode={displayMode}
            resetTimerDisplayMode={resetTimerDisplayMode}
            onResetTimerDisplayModeToggle={onResetTimerDisplayModeToggle}
          />
          {entry.perMachine.length > 1 && (
            <div className="flex flex-wrap gap-1 px-3 -mt-2 mb-2">
              <span className="text-[10px] text-muted-foreground mr-1">
                Combined from {entry.perMachine.length} machines:
              </span>
              {entry.perMachine.map((src) => (
                <MachineBadge
                  key={src.machine.id}
                  name={src.machine.name}
                  isLocal={src.machine.isLocal}
                />
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  )
}
