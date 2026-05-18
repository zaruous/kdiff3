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

// ── LCS core ─────────────────────────────────────────────────────────────────

function lcsBacktrack(a: string[], b: string[]): Array<'equal' | 'delete' | 'insert'> {
  const m = a.length, n = b.length
  // Cap for very large files to avoid OOM
  if (m > 8000 || n > 8000) return largeFileDiff(a, b)

  const dp = new Int32Array((m + 1) * (n + 1))
  for (let i = m - 1; i >= 0; i--) {
    for (let j = n - 1; j >= 0; j--) {
      const k = i * (n + 1) + j
      dp[k] = a[i] === b[j]
        ? 1 + dp[(i + 1) * (n + 1) + (j + 1)]
        : Math.max(dp[(i + 1) * (n + 1) + j], dp[i * (n + 1) + (j + 1)])
    }
  }

  const ops: Array<'equal' | 'delete' | 'insert'> = []
  let i = 0, j = 0
  while (i < m || j < n) {
    if (i < m && j < n && a[i] === b[j]) {
      ops.push('equal'); i++; j++
    } else if (j < n && (i >= m || dp[i * (n + 1) + (j + 1)] >= dp[(i + 1) * (n + 1) + j])) {
      ops.push('insert'); j++
    } else {
      ops.push('delete'); i++
    }
  }
  return ops
}

// Fast heuristic for large files: split into equal prefix/suffix, diff middle
function largeFileDiff(a: string[], b: string[]): Array<'equal' | 'delete' | 'insert'> {
  let lo = 0
  while (lo < a.length && lo < b.length && a[lo] === b[lo]) lo++
  let ai = a.length - 1, bi = b.length - 1
  while (ai > lo && bi > lo && a[ai] === b[bi]) { ai--; bi-- }

  const prefix: Array<'equal' | 'delete' | 'insert'> = Array(lo).fill('equal')
  const midA = a.slice(lo, ai + 1), midB = b.slice(lo, bi + 1)
  const mid = midA.map(() => 'delete' as const).concat(midB.map(() => 'insert' as const))
  const suffix: Array<'equal' | 'delete' | 'insert'> = Array(a.length - ai - 1).fill('equal')
  return [...prefix, ...mid, ...suffix]
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
  const ops = lcsBacktrack(a, b)

  const lines: DiffLine[] = []
  let ia = 0, ib = 0

  // Group consecutive ops to pair deletes+inserts as 'changed'
  let p = 0
  while (p < ops.length) {
    if (ops[p] === 'equal') {
      lines.push({ type: 'equal', numA: ia + 1, numB: ib + 1, textA: rawA[ia], textB: rawB[ib] })
      ia++; ib++; p++
    } else {
      // Collect a block of deletes and inserts
      let delCount = 0, insCount = 0
      const blockStart = p
      while (p < ops.length && ops[p] !== 'equal') {
        if (ops[p] === 'delete') delCount++
        else insCount++
        p++
      }
      // Pair them as 'changed', remainder as one-sided
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
      void blockStart
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
  const ops = lcsBacktrack(wa, wb)

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
