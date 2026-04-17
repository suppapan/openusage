/** Wire types for multi-machine sync (mirrors openusage-shared Rust types). */

import type { MetricLine } from "@/lib/plugin-types"

export type PluginSnapshot = {
  providerId: string
  displayName: string
  plan?: string
  lines: MetricLine[]
  fetchedAt: string
}

export type RemoteMachine = {
  machineId: string
  machineName: string
  snapshots: PluginSnapshot[]
  pushedAt: string
  lastSeenAt: string
}

export type SyncPullResponse = {
  machines: RemoteMachine[]
}
