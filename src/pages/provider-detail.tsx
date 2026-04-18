import { useState } from "react"
import { ProviderCard } from "@/components/provider-card"
import type { PluginDisplayState } from "@/lib/plugin-types"
import type { DisplayMode, ResetTimerDisplayMode } from "@/lib/settings"
import { useAppSyncStore } from "@/stores/app-sync-store"

interface ProviderDetailPageProps {
  plugin: PluginDisplayState | null
  onRetry?: () => void
  displayMode: DisplayMode
  resetTimerDisplayMode: ResetTimerDisplayMode
  onResetTimerDisplayModeToggle?: () => void
}

const LOCAL_TAB_ID = "__local__"

export function ProviderDetailPage({
  plugin,
  onRetry,
  displayMode,
  resetTimerDisplayMode,
  onResetTimerDisplayModeToggle,
}: ProviderDetailPageProps) {
  const { syncEnabled, remoteMachines } = useAppSyncStore()
  const [activeMachineId, setActiveMachineId] = useState<string>(LOCAL_TAB_ID)

  if (!plugin) {
    return (
      <div className="text-center text-muted-foreground py-8">
        Provider not found
      </div>
    )
  }

  // Build the list of machines that have this provider
  const remoteSources = syncEnabled
    ? remoteMachines
        .map((m) => {
          const snap = m.snapshots.find((s) => s.providerId === plugin.meta.id)
          return snap ? { machine: m, snapshot: snap } : null
        })
        .filter((x): x is NonNullable<typeof x> => x !== null)
    : []

  const tabs: Array<{ id: string; label: string }> = [
    { id: LOCAL_TAB_ID, label: "This machine" },
    ...remoteSources.map((r) => ({ id: r.machine.machineId, label: r.machine.machineName })),
  ]

  const activeRemote = remoteSources.find((r) => r.machine.machineId === activeMachineId)

  return (
    <div>
      {remoteSources.length > 0 && (
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

      {activeMachineId === LOCAL_TAB_ID && (
        <ProviderCard
          name={plugin.meta.name}
          plan={plugin.data?.plan}
          links={plugin.meta.links}
          showSeparator={false}
          loading={plugin.loading}
          error={plugin.error}
          lines={plugin.data?.lines ?? []}
          skeletonLines={plugin.meta.lines}
          lastManualRefreshAt={plugin.lastManualRefreshAt}
          onRetry={onRetry}
          scopeFilter="all"
          displayMode={displayMode}
          resetTimerDisplayMode={resetTimerDisplayMode}
          onResetTimerDisplayModeToggle={onResetTimerDisplayModeToggle}
        />
      )}

      {activeRemote && (
        <ProviderCard
          name={activeRemote.snapshot.displayName}
          plan={activeRemote.snapshot.plan}
          links={plugin.meta.links}
          showSeparator={false}
          loading={false}
          error={null}
          lines={activeRemote.snapshot.lines}
          skeletonLines={plugin.meta.lines}
          lastManualRefreshAt={null}
          scopeFilter="all"
          displayMode={displayMode}
          resetTimerDisplayMode={resetTimerDisplayMode}
          onResetTimerDisplayModeToggle={onResetTimerDisplayModeToggle}
        />
      )}
    </div>
  )
}
