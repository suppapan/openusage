import { describe, expect, it } from "vitest"
import { combineMetricLines, sumNumericTextValues } from "@/lib/aggregate-metrics"
import type { MetricLine } from "@/lib/plugin-types"

const local = { id: "__local__", name: "This machine", isLocal: true }
const remote = { id: "remote-1", name: "ubuntu-server", isLocal: false }

describe("combineMetricLines", () => {
  it("sums count-format progress used across machines", () => {
    const lineA: MetricLine = {
      type: "progress",
      label: "Tokens today",
      used: 1_000_000,
      limit: 5_000_000,
      format: { kind: "count", suffix: "tokens" },
    }
    const lineB: MetricLine = { ...lineA, used: 2_500_000 }

    const out = combineMetricLines([
      { machine: local, lines: [lineA] },
      { machine: remote, lines: [lineB] },
    ])

    expect(out).toHaveLength(1)
    expect(out[0].type).toBe("progress")
    if (out[0].type === "progress") {
      expect(out[0].used).toBe(3_500_000)
      expect(out[0].limit).toBe(5_000_000)
    }
  })

  it("sums dollar-format progress used across machines", () => {
    const lineA: MetricLine = {
      type: "progress",
      label: "Spend today",
      used: 4.25,
      limit: 50,
      format: { kind: "dollars" },
    }
    const lineB: MetricLine = { ...lineA, used: 11.75 }

    const out = combineMetricLines([
      { machine: local, lines: [lineA] },
      { machine: remote, lines: [lineB] },
    ])

    if (out[0].type !== "progress") throw new Error("expected progress")
    expect(out[0].used).toBe(16)
    expect(out[0].limit).toBe(50)
  })

  it("takes max of percent-format progress (per-account counters)", () => {
    const lineA: MetricLine = {
      type: "progress",
      label: "Session",
      used: 50,
      limit: 100,
      format: { kind: "percent" },
    }
    const lineB: MetricLine = { ...lineA, used: 85 }

    const out = combineMetricLines([
      { machine: local, lines: [lineA] },
      { machine: remote, lines: [lineB] },
    ])

    if (out[0].type !== "progress") throw new Error("expected progress")
    expect(out[0].used).toBe(85)
  })

  it("dedupes text lines with identical values", () => {
    const line: MetricLine = { type: "text", label: "Plan", value: "Pro" }

    const out = combineMetricLines([
      { machine: local, lines: [line] },
      { machine: remote, lines: [line] },
    ])

    expect(out).toHaveLength(1)
    if (out[0].type !== "text") throw new Error("expected text")
    expect(out[0].value).toBe("Pro")
  })

  it("concatenates differing text values with +", () => {
    const a: MetricLine = { type: "text", label: "Last call", value: "10s ago" }
    const b: MetricLine = { type: "text", label: "Last call", value: "1m ago" }

    const out = combineMetricLines([
      { machine: local, lines: [a] },
      { machine: remote, lines: [b] },
    ])

    if (out[0].type !== "text") throw new Error("expected text")
    expect(out[0].value).toBe("10s ago + 1m ago")
  })

  it("preserves label order from the first machine", () => {
    const a1: MetricLine = { type: "text", label: "First", value: "A" }
    const a2: MetricLine = { type: "text", label: "Second", value: "B" }
    const b: MetricLine = { type: "text", label: "Second", value: "C" }

    const out = combineMetricLines([
      { machine: local, lines: [a1, a2] },
      { machine: remote, lines: [b] },
    ])

    expect(out.map((l) => l.label)).toEqual(["First", "Second"])
  })

  it("sums dollar+token text values like ccusage 'Today' lines", () => {
    const a: MetricLine = { type: "text", label: "Today", value: "$1.20 \u00b7 500K tokens" }
    const b: MetricLine = { type: "text", label: "Today", value: "$0.80 \u00b7 1.5M tokens" }

    const out = combineMetricLines([
      { machine: local, lines: [a] },
      { machine: remote, lines: [b] },
    ])

    if (out[0].type !== "text") throw new Error("expected text")
    expect(out[0].value).toBe("$2.00 \u00b7 2M tokens")
  })

  it("sums dollar-only text values", () => {
    expect(sumNumericTextValues(["$1.50", "$2.25", "$0.75"])).toBe("$4.50")
  })

  it("sums token-only text values across K/M units", () => {
    expect(sumNumericTextValues(["500K tokens", "1.5M tokens"])).toBe("2M tokens")
  })

  it("returns null when a value can't be parsed as dollars or tokens", () => {
    expect(sumNumericTextValues(["12 calls", "$3.00"])).toBe(null)
  })

  it("returns the original line when only one machine reports it", () => {
    const a: MetricLine = { type: "text", label: "Only on local", value: "42" }
    const b: MetricLine = { type: "text", label: "Only on remote", value: "99" }

    const out = combineMetricLines([
      { machine: local, lines: [a] },
      { machine: remote, lines: [b] },
    ])

    expect(out).toHaveLength(2)
    expect(out.find((l) => l.label === "Only on local")?.type).toBe("text")
    expect(out.find((l) => l.label === "Only on remote")?.type).toBe("text")
  })
})
