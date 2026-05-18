import { computeLineDiff, DiffLine } from './diff'

export type ResolvedChoice = 'A' | 'B' | 'C'

export type MergeDetail =
  | 'NoChange'
  | 'BChanged' | 'CChanged'
  | 'BCChanged' | 'BCEqual'
  | 'BAdded' | 'CAdded' | 'BCAdded' | 'BCAddedEqual'
  | 'BDeleted' | 'CDeleted' | 'BCDeleted'
  | 'Conflict'

export interface MergeBlock {
  detail: MergeDetail
  linesA: string[]
  linesB: string[]
  linesC: string[]
  resolved?: ResolvedChoice
  // source line numbers (1-indexed, for display)
  startA: number
  startB: number
  startC: number
}

export interface MergeResult {
  blocks: MergeBlock[]
  conflictCount: number
  unresolvedCount: number
}

function diffToSpans(diff: DiffLine[]): { aIdx: number, bIdx: number, equal: boolean, len: number }[] {
  const spans = []
  let aIdx = 0, bIdx = 0, p = 0
  while (p < diff.length) {
    const d = diff[p]
    if (d.type === 'equal') {
      let len = 0
      while (p < diff.length && diff[p].type === 'equal') { len++; p++ }
      spans.push({ aIdx, bIdx, equal: true, len })
      aIdx += len; bIdx += len
    } else {
      let da = 0, db = 0
      while (p < diff.length && diff[p].type !== 'equal') {
        if (diff[p].type === 'delete') da++
        else if (diff[p].type === 'insert') db++
        else { da++; db++ } // changed
        p++
      }
      if (da > 0 || db > 0) spans.push({ aIdx, bIdx, equal: false, len: Math.max(da, db), _da: da, _db: db } as any)
      aIdx += da; bIdx += db
    }
  }
  return spans
}

export function buildMerger(
  linesA: string[],
  linesB: string[],
  linesC: string[]
): MergeResult {
  const diffAB = computeLineDiff(linesA, linesB)
  const diffAC = linesC.length > 0 ? computeLineDiff(linesA, linesC) : null

  if (!diffAC) return buildTwoway(linesA, linesB, diffAB)

  const spansAB = diffToSpans(diffAB)
  const spansAC = diffToSpans(diffAC)

  const blocks: MergeBlock[] = []

  // Build set of A-indices changed in AB and AC
  const abChanged = new Set<number>()
  const acChanged = new Set<number>()

  for (const d of diffAB) {
    if (d.type === 'delete' || d.type === 'changed') abChanged.add(d.numA! - 1)
  }
  for (const d of diffAC) {
    if (d.type === 'delete' || d.type === 'changed') acChanged.add(d.numA! - 1)
  }

  let ai = 0, bOff = 0, cOff = 0
  const aLen = linesA.length
  for (const span of spansAB) {
    // Equal prefix before this span
    while (ai < span.aIdx) {
      // Check if C changed here too
      if (acChanged.has(ai)) {
        // C changed, A=B here
        // flush as CChanged
        const startA = ai, startB = bOff, startC = cOff
        ai++; bOff++; cOff++
        blocks.push({
          detail: 'CChanged',
          linesA: [linesA[startA]],
          linesB: [linesB[startB]],
          linesC: cOff <= linesC.length ? [linesC[startC]] : [],
          startA: startA + 1, startB: startB + 1, startC: startC + 1
        })
      } else {
        // true equal
        const startA = ai, startB = bOff, startC = cOff
        let len = 0
        while (ai < span.aIdx && !acChanged.has(ai)) { len++; ai++; bOff++; cOff++ }
        if (len > 0)
          blocks.push({
            detail: 'NoChange',
            linesA: linesA.slice(startA, startA + len),
            linesB: linesB.slice(startB, startB + len),
            linesC: linesC.slice(startC, startC + len),
            startA: startA + 1, startB: startB + 1, startC: startC + 1
          })
      }
    }

    if (!span.equal) {
      const spanAny = span as any
      const da: number = spanAny._da !== undefined ? spanAny._da : span.len
      const db: number = spanAny._db !== undefined ? spanAny._db : span.len
      // Check if A's lines in this span are also changed in C
      let cChanged = false
      for (let k = 0; k < da; k++) if (acChanged.has(span.aIdx + k)) { cChanged = true; break }

      const startA = ai, startB = bOff, startC = cOff
      const la = linesA.slice(startA, startA + da)
      const lb = linesB.slice(startB, startB + db)
      const lc = linesC.slice(startC, startC + da)

      let detail: MergeDetail
      if (!cChanged) {
        detail = db === 0 ? 'BDeleted' : da === 0 ? 'BAdded' : 'BChanged'
      } else {
        // Both changed
        const bcEqual = JSON.stringify(lb) === JSON.stringify(lc)
        detail = bcEqual ? 'BCEqual' : 'Conflict'
      }

      blocks.push({ detail, linesA: la, linesB: lb, linesC: lc, startA: startA + 1, startB: startB + 1, startC: startC + 1 })
      ai += da; bOff += db; cOff += cChanged ? da : da
    }
  }

  // Remaining equal lines
  if (ai < aLen) {
    blocks.push({
      detail: 'NoChange',
      linesA: linesA.slice(ai),
      linesB: linesB.slice(bOff),
      linesC: linesC.slice(cOff),
      startA: ai + 1, startB: bOff + 1, startC: cOff + 1
    })
  }

  const conflictCount = blocks.filter(b => b.detail === 'Conflict').length
  return { blocks, conflictCount, unresolvedCount: conflictCount }
}

function buildTwoway(linesA: string[], linesB: string[], diff: DiffLine[]): MergeResult {
  const blocks: MergeBlock[] = []
  let ia = 0, ib = 0

  // Group consecutive equal lines
  let p = 0
  while (p < diff.length) {
    if (diff[p].type === 'equal') {
      const startA = ia, startB = ib
      let len = 0
      while (p < diff.length && diff[p].type === 'equal') { len++; p++; ia++; ib++ }
      blocks.push({ detail: 'NoChange', linesA: linesA.slice(startA, startA + len), linesB: linesB.slice(startB, startB + len), linesC: [], startA: startA + 1, startB: startB + 1, startC: 1 })
    } else {
      const startA = ia, startB = ib
      let da = 0, db = 0
      while (p < diff.length && diff[p].type !== 'equal') {
        if (diff[p].type === 'delete') da++
        else if (diff[p].type === 'insert') db++
        else { da++; db++ }
        p++
      }
      const detail: MergeDetail = da === 0 ? 'BAdded' : db === 0 ? 'BDeleted' : 'BChanged'
      blocks.push({ detail, linesA: linesA.slice(startA, startA + da), linesB: linesB.slice(startB, startB + db), linesC: [], startA: startA + 1, startB: startB + 1, startC: 1 })
      ia += da; ib += db
    }
  }

  const conflictCount = 0
  return { blocks, conflictCount, unresolvedCount: 0 }
}

export function resolveBlock(result: MergeResult, idx: number, choice: ResolvedChoice): MergeResult {
  const blocks = result.blocks.map((b, i) => i === idx ? { ...b, resolved: choice } : b)
  const unresolvedCount = blocks.filter(b => b.detail === 'Conflict' && !b.resolved).length
  return { ...result, blocks, unresolvedCount }
}

export function generateOutput(result: MergeResult): string {
  const lines: string[] = []
  for (const block of result.blocks) {
    if (block.detail === 'NoChange') {
      lines.push(...block.linesA)
    } else if (block.detail === 'BChanged' || block.detail === 'BAdded' || block.detail === 'BDeleted' || block.detail === 'BCEqual') {
      lines.push(...block.linesB)
    } else if (block.detail === 'CChanged' || block.detail === 'CAdded' || block.detail === 'CDeleted') {
      lines.push(...block.linesC)
    } else if (block.detail === 'Conflict') {
      if (block.resolved === 'A') lines.push(...block.linesA)
      else if (block.resolved === 'B') lines.push(...block.linesB)
      else if (block.resolved === 'C') lines.push(...block.linesC)
      else {
        lines.push('<<<<<<< A')
        lines.push(...block.linesA)
        lines.push('||||||| B')
        lines.push(...block.linesB)
        lines.push('======= C')
        lines.push(...block.linesC)
        lines.push('>>>>>>>')
      }
    }
  }
  return lines.join('\n')
}

export function nextConflict(result: MergeResult, from: number): number {
  for (let i = from + 1; i < result.blocks.length; i++)
    if (result.blocks[i].detail === 'Conflict' && !result.blocks[i].resolved) return i
  for (let i = 0; i <= from; i++)
    if (result.blocks[i].detail === 'Conflict' && !result.blocks[i].resolved) return i
  return -1
}

export function prevConflict(result: MergeResult, from: number): number {
  for (let i = from - 1; i >= 0; i--)
    if (result.blocks[i].detail === 'Conflict' && !result.blocks[i].resolved) return i
  for (let i = result.blocks.length - 1; i > from; i--)
    if (result.blocks[i].detail === 'Conflict' && !result.blocks[i].resolved) return i
  return -1
}
