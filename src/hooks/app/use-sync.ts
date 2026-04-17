import { useEffect, useRef } from "react"
import { invoke, isTauri } from "@tauri-apps/api/core"
import { useAppSyncStore } from "@/stores/app-sync-store"
import type { SyncPullResponse } from "@/lib/sync-types"

const SYNC_POLL_INTERVAL_MS = 60_000 // 60 seconds

export function useSync() {
  const { syncEnabled, syncToken, relayUrl, setRemoteMachines } = useAppSyncStore()
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    if (!isTauri() || !syncEnabled || !syncToken || !relayUrl) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current)
        intervalRef.current = null
      }
      return
    }

    const pull = async () => {
      try {
        const response = await invoke<SyncPullResponse>("pull_remote_machines", {
          relayUrl,
          token: syncToken,
        })
        setRemoteMachines(response.machines)
      } catch (e) {
        console.error("sync pull failed:", e)
      }
    }

    // Pull immediately on enable
    void pull()

    intervalRef.current = setInterval(pull, SYNC_POLL_INTERVAL_MS)

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current)
        intervalRef.current = null
      }
    }
  }, [syncEnabled, syncToken, relayUrl, setRemoteMachines])
}
