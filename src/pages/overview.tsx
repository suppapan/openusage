import { useState } from "react"
import { ProviderCard } from "@/components/provider-card"
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

const LOCAL_TAB_ID = "__local__"

export function OverviewPage({
  plugins,
  onRetryPlugin,
  displayMode,
  resetTimerDisplayMode,
  onResetTimerDisplayModeToggle,
}: OverviewPageProps) {
  const { syncEnabled, remoteMachines } = useAppSyncStore()
  const [activeMachineId, setActiveMachineId] = useState<string>(LOCAL_TAB_ID)

  const hasRemoteMachines = syncEnabled && remoteMachines.length > 0

  if (plugins.length === 0 && !hasRemoteMachines) {
    return (
      <div className="text-center text-muted-foreground py-8">
        No providers enabled
      </div>
    )
  }

  // Build the tab list: Local first, then one per remote machine
  const tabs: Array<{ id: string; label: string }> = [
    { id: LOCAL_TAB_ID, label: "This machine" },
    ...remoteMachines.map((m) => ({ id: m.machineId, label: m.machineName })),
  ]

  // Find the snapshot data for the active tab
  const activeRemoteMachine = remoteMachines.find((m) => m.machineId === activeMachineId)

  return (
    <div>
      {hasRemoteMachines && (
        <div className="flex flex-wrap gap-1 mb-2 px-0.5">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveMachineId(tab.id)}
              className={`text-xs px-2 py-0.5 rounded-full transition-colors ${
                activeMachineId === tab.id
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground hover:bg-muted/80"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      )}

      {/* Local machine: render the live plugin states */}
      {activeMachineId === LOCAL_TAB_ID && plugins.map((plugin, index) => (
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

      {/* Remote machine: render its snapshots */}
      {activeRemoteMachine && (() => {
        if (activeRemoteMachine.snapshots.length === 0) {
          return (
            <div className="text-center text-muted-foreground py-8 text-sm">
              No providers reporting yet from this machine.
            </div>
          )
        }
        return activeRemoteMachine.snapshots.map((snap, index) => {
          // If the local app has metadata for this provider, use it for scope
          // filtering and skeleton rendering. Otherwise show all lines.
          const meta = plugins.find((p) => p.meta.id === snap.providerId)?.meta ?? null
          return (
            <ProviderCard
              key={snap.providerId}
              name={snap.displayName}
              plan={snap.plan}
              showSeparator={index < activeRemoteMachine.snapshots.length - 1}
              loading={false}
              error={null}
              lines={snap.lines}
              skeletonLines={meta?.lines ?? []}
              lastManualRefreshAt={null}
              scopeFilter={meta ? "overview" : "all"}
              displayMode={displayMode}
              resetTimerDisplayMode={resetTimerDisplayMode}
              onResetTimerDisplayModeToggle={onResetTimerDisplayModeToggle}
            />
          )
        })
      })()}
    </div>
  )
}
