import { useCallback, useState } from "react"
import { invoke, isTauri } from "@tauri-apps/api/core"
import { useShallow } from "zustand/react/shallow"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { useAppSyncStore } from "@/stores/app-sync-store"
import { MachineBadge } from "@/components/machine-badge"
import { saveSettingToStore } from "@/lib/settings"

export function SyncSettings() {
  const {
    syncEnabled,
    setSyncEnabled,
    syncToken,
    setSyncToken,
    relayUrl,
    setRelayUrl,
    remoteMachines,
  } = useAppSyncStore(
    useShallow((s) => ({
      syncEnabled: s.syncEnabled,
      setSyncEnabled: s.setSyncEnabled,
      syncToken: s.syncToken,
      setSyncToken: s.setSyncToken,
      relayUrl: s.relayUrl,
      setRelayUrl: s.setRelayUrl,
      remoteMachines: s.remoteMachines,
    }))
  )

  const [copied, setCopied] = useState(false)
  const [relayInput, setRelayInput] = useState(relayUrl)

  const handleToggleSync = useCallback(async (checked: boolean) => {
    setSyncEnabled(checked)
    await saveSettingToStore("syncEnabled", checked)
  }, [setSyncEnabled])

  const handleGenerateToken = useCallback(async () => {
    if (!isTauri()) return
    try {
      const token = await invoke<string>("generate_sync_token")
      setSyncToken(token)
    } catch (e) {
      console.error("failed to generate token:", e)
    }
  }, [setSyncToken])

  const handleRevokeToken = useCallback(async () => {
    if (!isTauri()) return
    try {
      await invoke("revoke_sync_token")
      setSyncToken(null)
    } catch (e) {
      console.error("failed to revoke token:", e)
    }
  }, [setSyncToken])

  const handleCopyToken = useCallback(async () => {
    if (!syncToken) return
    await navigator.clipboard.writeText(syncToken)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }, [syncToken])

  const handleRelayUrlBlur = useCallback(async () => {
    const trimmed = relayInput.trim()
    setRelayUrl(trimmed)
    await saveSettingToStore("syncRelayUrl", trimmed)
  }, [relayInput, setRelayUrl])

  return (
    <section>
      <h3 className="text-lg font-semibold mb-0">Multi-Machine Sync</h3>
      <p className="text-sm text-muted-foreground mb-2">
        Aggregate usage from multiple machines
      </p>

      <div className="space-y-3">
        <label className="flex items-center gap-2 cursor-pointer">
          <Checkbox
            checked={syncEnabled}
            onCheckedChange={handleToggleSync}
          />
          <span className="text-sm">Enable sync</span>
        </label>

        {syncEnabled && (
          <>
            <div>
              <label className="text-xs font-medium text-muted-foreground block mb-1">
                Relay URL
              </label>
              <input
                type="text"
                value={relayInput}
                onChange={(e) => setRelayInput(e.target.value)}
                onBlur={handleRelayUrlBlur}
                placeholder="https://relay.example.com:8090"
                className="w-full rounded-md border bg-background px-2.5 py-1.5 text-sm"
              />
            </div>

            <div>
              <label className="text-xs font-medium text-muted-foreground block mb-1">
                Sync Token
              </label>
              {syncToken ? (
                <div className="flex items-center gap-2">
                  <code className="flex-1 rounded-md border bg-muted/50 px-2.5 py-1.5 text-xs font-mono truncate">
                    {syncToken}
                  </code>
                  <Button size="sm" variant="outline" onClick={handleCopyToken}>
                    {copied ? "Copied" : "Copy"}
                  </Button>
                  <Button size="sm" variant="outline" onClick={handleRevokeToken}>
                    Revoke
                  </Button>
                </div>
              ) : (
                <Button size="sm" onClick={handleGenerateToken}>
                  Generate Token
                </Button>
              )}
              {syncToken && (
                <p className="text-[10px] text-muted-foreground mt-1">
                  Run on remote machines: <code className="bg-muted/50 px-1 rounded">openusage-agent --token {syncToken} --relay {relayUrl || "<RELAY_URL>"}</code>
                </p>
              )}
            </div>

            {remoteMachines.length > 0 && (
              <div>
                <label className="text-xs font-medium text-muted-foreground block mb-1">
                  Connected Machines ({remoteMachines.length})
                </label>
                <div className="flex flex-wrap gap-1">
                  {remoteMachines.map((m) => (
                    <MachineBadge
                      key={m.machineId}
                      name={m.machineName}
                      lastSeenAt={m.lastSeenAt}
                    />
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </section>
  )
}
