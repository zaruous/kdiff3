export interface DiffOptions {
  ignoreWhitespace: boolean
  ignoreCase: boolean
}

export type ChangeType = 'equal' | 'delete' | 'insert' | 'changed'

export interface DiffLine {
  type: ChangeType
  numA?: number
  numB?: number
  textA: string
  textB: string
}

export interface WordToken {
  type: 'equal' | 'delete' | 'insert'
  value: string
}

export interface DiffStats {
  additions: number
  deletions: number
  changes: number
}

// ── Myers O(n·d) diff ──────────────────────────────────────────────────────────
// Time O(n·d), space O(n+m) + O(d²) for compact trace.
// Falls back to naive heuristic when d > D_LIMIT.

type Op = 'equal' | 'delete' | 'insert'

const D_LIMIT = 2000

function myersDiff(a: string[], b: string[]): Op[] {
  const n = a.length, m = b.length

  // Strip common prefix
  let lo = 0
  while (lo < n && lo < m && a[lo] === b[lo]) lo++
  if (lo === n && lo === m) return new Array<Op>(n).fill('equal')

  // Strip common suffix
  let hiA = n, hiB = m
  while (hiA > lo && hiB > lo && a[hiA - 1] === b[hiB - 1]) { hiA--; hiB-- }

  const A = a.slice(lo, hiA), B = b.slice(lo, hiB)
  const N = A.length, M = B.length
  const pre: Op[] = new Array(lo).fill('equal')
  const suf: Op[] = new Array(n - hiA).fill('equal')

  if (N === 0) return [...pre, ...new Array<Op>(M).fill('insert'), ...suf]
  if (M === 0) return [...pre, ...new Array<Op>(N).fill('delete'), ...suf]

  const maxD = N + M
  const off = maxD + 1
  const v = new Int32Array(2 * maxD + 4)
  v[off + 1] = 0

  // parityTrace[d] = V[k] for k ∈ {-d, -d+2, ..., d} AFTER processing depth d.
  // At depth d, valid k's have parity d. For backtracking at depth d we need
  // parityTrace[d-1] (parity d-1 values) to reconstruct the move direction.
  // Total storage: sum_{d=0}^{D-1} (d+1) = D*(D+1)/2 Int32 values ≤ 2MB at D=2000.
  const pt: Int32Array[] = []

  let ed = -1
  outer: for (let d = 0; d <= maxD; d++) {
    if (d > D_LIMIT) break
    for (let k = -d; k <= d; k += 2) {
      const dn = k === -d || (k !== d && v[off + k - 1] < v[off + k + 1])
      let x = dn ? v[off + k + 1] : v[off + k - 1] + 1
      let y = x - k
      while (x < N && y < M && A[x] === B[y]) { x++; y++ }
      v[off + k] = x
      if (x >= N && y >= M) { ed = d; break outer }
    }
    // Snapshot: saved after depth d, used for backtracking at depth d+1.
    // Index i = (k + d) >> 1 maps k ∈ {-d, -d+2, ..., d} → i ∈ {0, 1, ..., d}.
    const snap = new Int32Array(d + 1)
    for (let k = -d, i = 0; k <= d; k += 2, i++) snap[i] = v[off + k]
    pt.push(snap)
  }

  if (ed < 0) {
    // Edit distance exceeded budget; fall back to all-delete then all-insert.
    return [...pre, ...new Array<Op>(N).fill('delete'), ...new Array<Op>(M).fill('insert'), ...suf]
  }

  // Backtrack through compact trace to reconstruct edit script in reverse.
  const ops: Op[] = []
  let x = N, y = M
  for (let d = ed; d > 0; d--) {
    // snap = pt[d-1] holds parity-(d-1) values in range [-(d-1), d-1].
    // Index for k': i = (k' + d - 1) >> 1
    const snap = pt[d - 1]
    const k = x - y
    const vm1 = k > -d ? snap[(k - 1 + d - 1) >> 1] : -1
    const vp1 = k <  d ? snap[(k + 1 + d - 1) >> 1] : -1
    const dn = k === -d || (k !== d && vm1 < vp1)
    const px = dn ? vp1 : vm1
    const pk = dn ? k + 1 : k - 1
    const py = px - pk
    // Undo snake from (dn ? px : px+1, midY) → (x, y), then undo the edit.
    for (let i = (dn ? px : px + 1); i < x; i++) ops.push('equal')
    ops.push(dn ? 'insert' : 'delete')
    x = px; y = py
  }
  // Leading snake at edit depth 0
  for (let i = 0; i < x; i++) ops.push('equal')
  ops.reverse()

  return [...pre, ...ops, ...suf]
}

// ── Public API ────────────────────────────────────────────────────────────────

function normalize(line: string, opts?: DiffOptions): string {
  let s = opts?.ignoreCase ? line.toLowerCase() : line
  if (opts?.ignoreWhitespace) s = s.trim().replace(/\s+/g, ' ')
  return s
}

export function computeLineDiff(
  rawA: string[],
  rawB: string[],
  opts?: DiffOptions
): DiffLine[] {
  const a = rawA.map(l => normalize(l, opts))
  const b = rawB.map(l => normalize(l, opts))
  const ops = myersDiff(a, b)

  const lines: DiffLine[] = []
  let ia = 0, ib = 0, p = 0

  while (p < ops.length) {
    if (ops[p] === 'equal') {
      lines.push({ type: 'equal', numA: ia + 1, numB: ib + 1, textA: rawA[ia], textB: rawB[ib] })
      ia++; ib++; p++
    } else {
      let delCount = 0, insCount = 0
      while (p < ops.length && ops[p] !== 'equal') {
        if (ops[p] === 'delete') delCount++
        else insCount++
        p++
      }
      const paired = Math.min(delCount, insCount)
      for (let k = 0; k < paired; k++) {
        lines.push({ type: 'changed', numA: ia + k + 1, numB: ib + k + 1, textA: rawA[ia + k], textB: rawB[ib + k] })
      }
      for (let k = paired; k < delCount; k++) {
        lines.push({ type: 'delete', numA: ia + k + 1, textA: rawA[ia + k], textB: '' })
      }
      for (let k = paired; k < insCount; k++) {
        lines.push({ type: 'insert', numB: ib + k + 1, textA: '', textB: rawB[ib + k] })
      }
      ia += delCount; ib += insCount
    }
  }
  return lines
}

export function computeStats(lines: DiffLine[]): DiffStats {
  let additions = 0, deletions = 0, changes = 0
  for (const l of lines) {
    if (l.type === 'insert') additions++
    else if (l.type === 'delete') deletions++
    else if (l.type === 'changed') changes++
  }
  return { additions, deletions, changes }
}

// ── Word-level diff ───────────────────────────────────────────────────────────

function tokenizeWords(text: string): string[] {
  return text.match(/\S+|\s+/g) ?? (text.length ? [text] : [])
}

export function computeWordDiff(textA: string, textB: string): { a: WordToken[], b: WordToken[] } {
  const wa = tokenizeWords(textA)
  const wb = tokenizeWords(textB)
  const ops = myersDiff(wa, wb)

  const tokA: WordToken[] = []
  const tokB: WordToken[] = []
  let ia = 0, ib = 0
  for (const op of ops) {
    if (op === 'equal')  { tokA.push({ type: 'equal', value: wa[ia] }); tokB.push({ type: 'equal', value: wb[ib] }); ia++; ib++ }
    else if (op === 'delete') { tokA.push({ type: 'delete', value: wa[ia] }); ia++ }
    else { tokB.push({ type: 'insert', value: wb[ib] }); ib++ }
  }
  return { a: tokA, b: tokB }
}
