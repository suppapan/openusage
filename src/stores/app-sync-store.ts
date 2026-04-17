import { create } from "zustand"
import type { RemoteMachine } from "@/lib/sync-types"

type AppSyncStore = {
  syncEnabled: boolean
  syncToken: string | null
  relayUrl: string
  remoteMachines: RemoteMachine[]
  setSyncEnabled: (value: boolean) => void
  setSyncToken: (value: string | null) => void
  setRelayUrl: (value: string) => void
  setRemoteMachines: (machines: RemoteMachine[]) => void
}

export const useAppSyncStore = create<AppSyncStore>((set) => ({
  syncEnabled: false,
  syncToken: null,
  relayUrl: "",
  remoteMachines: [],
  setSyncEnabled: (value) => set({ syncEnabled: value }),
  setSyncToken: (value) => set({ syncToken: value }),
  setRelayUrl: (value) => set({ relayUrl: value }),
  setRemoteMachines: (machines) => set({ remoteMachines: machines }),
}))
