import type { MetricLine, PluginDisplayState } from "@/lib/plugin-types"
import type { RemoteMachine, PluginSnapshot } from "@/lib/sync-types"

export type SourceMachine = {
  id: string
  name: string
  isLocal: boolean
}

export type CombinedProvider = {
  providerId: string
  displayName: string
  plan?: string
  meta: PluginDisplayState["meta"] | null
  /** Per-machine snapshots that contributed (for breakdown UI) */
  perMachine: Array<{ machine: SourceMachine; lines: MetricLine[] }>
  /** Aggregated lines (summed/maxed across machines as appropriate) */
  combinedLines: MetricLine[]
}

/**
 * Combine a single provider's metric lines from N machines.
 *
 * Rules:
 * - Progress lines with kind=percent: take MAX (per-account counters are
 *   already deduped server-side, so max gives the worst-case state)
 * - Progress lines with kind=dollars or kind=count: SUM `used` across
 *   machines, keep MAX `limit` (limits are per-plan, identical machines
 *   should match; if a machine has a higher limit, expose it)
 * - Text lines: if all machines have the same value, show once;
 *   otherwise concatenate "valueA + valueB"
 * - Badge lines: take the first occurrence (assume same per-account state)
 */
export function combineMetricLines(
  perMachine: Array<{ machine: SourceMachine; lines: MetricLine[] }>
): MetricLine[] {
  type Bucket = { line: MetricLine; sources: number }
  const byLabel = new Map<string, Bucket[]>()

  for (const { lines } of perMachine) {
    for (const line of lines) {
      const list = byLabel.get(line.label) ?? []
      list.push({ line, sources: 1 })
      byLabel.set(line.label, list)
    }
  }

  const out: MetricLine[] = []
  // Preserve label order from the first machine that had a given label
  const seen = new Set<string>()
  for (const { lines } of perMachine) {
    for (const line of lines) {
      if (seen.has(line.label)) continue
      seen.add(line.label)
      const buckets = byLabel.get(line.label) ?? []
      out.push(combineBucket(buckets.map((b) => b.line)))
    }
  }
  return out
}

function combineBucket(lines: MetricLine[]): MetricLine {
  if (lines.length === 1) return lines[0]
  const first = lines[0]

  if (first.type === "progress") {
    const all = lines.filter((l): l is Extract<MetricLine, { type: "progress" }> => l.type === "progress")
    const kind = first.format.kind
    if (kind === "percent") {
      // Take max — these are typically per-account, identical across machines
      const maxLine = all.reduce((acc, l) => (l.used > acc.used ? l : acc), all[0])
      return maxLine
    }
    // count / dollars: sum used, max limit
    const used = all.reduce((s, l) => s + (Number.isFinite(l.used) ? l.used : 0), 0)
    const limit = all.reduce((m, l) => Math.max(m, Number.isFinite(l.limit) ? l.limit : 0), 0)
    return { ...first, used, limit }
  }

  if (first.type === "text") {
    const all = lines.filter((l): l is Extract<MetricLine, { type: "text" }> => l.type === "text")
    const uniqValues = Array.from(new Set(all.map((l) => l.value)))
    if (uniqValues.length === 1) return first
    // Try to parse + sum numeric token/cost patterns (e.g. "$1.20 · 500K tokens")
    const summed = sumNumericTextValues(all.map((l) => l.value))
    if (summed) return { ...first, value: summed }
    return { ...first, value: uniqValues.join(" + ") }
  }

  // badge: take first
  return first
}

// ─── Text value summation helpers ───────────────────────────────────────────

/**
 * Parse a text value like "$1.20 · 500K tokens" or "$0.42 · 1.2M tokens" or
 * "1.5M tokens" or just "$3.45" and return any extractable dollar/token totals.
 * Returns null if neither dollars nor tokens are detectable.
 */
function parseDollarsAndTokens(value: string): { dollars: number | null; tokens: number | null } | null {
  const dollarMatch = value.match(/\$\s*(-?\d+(?:\.\d+)?)/)
  const tokenMatch = value.match(/(-?\d+(?:\.\d+)?)\s*([KMB])?\s*tokens?/i)
  let dollars: number | null = null
  let tokens: number | null = null
  if (dollarMatch) dollars = parseFloat(dollarMatch[1])
  if (tokenMatch) {
    const n = parseFloat(tokenMatch[1])
    const suffix = (tokenMatch[2] || "").toUpperCase()
    const mult = suffix === "B" ? 1e9 : suffix === "M" ? 1e6 : suffix === "K" ? 1e3 : 1
    tokens = n * mult
  }
  if (dollars === null && tokens === null) return null
  return { dollars, tokens }
}

function fmtTokens(n: number): string {
  const abs = Math.abs(n)
  if (abs >= 1e9) return (n / 1e9).toFixed(2).replace(/\.?0+$/, "") + "B"
  if (abs >= 1e6) return (n / 1e6).toFixed(2).replace(/\.?0+$/, "") + "M"
  if (abs >= 1e3) return (n / 1e3).toFixed(1).replace(/\.?0+$/, "") + "K"
  return String(Math.round(n))
}

/**
 * If every value is parseable as dollars and/or tokens, return a summed
 * formatted string like the original (e.g. "$3.20 · 1.5M tokens").
 * Returns null if any value can't be parsed (so the caller falls back to
 * concatenation).
 */
export function sumNumericTextValues(values: string[]): string | null {
  let totalDollars: number | null = null
  let totalTokens: number | null = null
  let anyDollars = false
  let anyTokens = false

  for (const v of values) {
    const parsed = parseDollarsAndTokens(v)
    if (!parsed) return null
    if (parsed.dollars !== null) {
      anyDollars = true
      totalDollars = (totalDollars ?? 0) + parsed.dollars
    }
    if (parsed.tokens !== null) {
      anyTokens = true
      totalTokens = (totalTokens ?? 0) + parsed.tokens
    }
  }

  const parts: string[] = []
  if (anyDollars && totalDollars !== null) parts.push("$" + totalDollars.toFixed(2))
  if (anyTokens && totalTokens !== null) parts.push(fmtTokens(totalTokens) + " tokens")
  return parts.length > 0 ? parts.join(" \u00b7 ") : null
}

/**
 * Build per-provider CombinedProvider entries from local plugin states + remote
 * machines, suitable for rendering in the "All Machines" view.
 */
export function combineByProvider(
  localPlugins: PluginDisplayState[],
  remoteMachines: RemoteMachine[]
): CombinedProvider[] {
  const localMachine: SourceMachine = { id: "__local__", name: "This machine", isLocal: true }

  const map = new Map<string, CombinedProvider>()

  // Seed with local plugins (preserves ordering from pluginSettings.order)
  for (const p of localPlugins) {
    if (!p.data) continue
    const entry: CombinedProvider = {
      providerId: p.meta.id,
      displayName: p.meta.name,
      plan: p.data.plan,
      meta: p.meta,
      perMachine: [{ machine: localMachine, lines: p.data.lines }],
      combinedLines: [],
    }
    map.set(p.meta.id, entry)
  }

  // Merge in remote snapshots
  for (const machine of remoteMachines) {
    const src: SourceMachine = {
      id: machine.machineId,
      name: machine.machineName,
      isLocal: false,
    }
    for (const snap of machine.snapshots) {
      let entry = map.get(snap.providerId)
      if (!entry) {
        entry = {
          providerId: snap.providerId,
          displayName: snap.displayName,
          plan: snap.plan,
          meta: null,
          perMachine: [],
          combinedLines: [],
        }
        map.set(snap.providerId, entry)
      }
      entry.perMachine.push({ machine: src, lines: snap.lines })
    }
  }

  for (const entry of map.values()) {
    entry.combinedLines = combineMetricLines(entry.perMachine)
  }

  return Array.from(map.values())
}
