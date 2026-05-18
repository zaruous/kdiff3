export type EntryStatus =
  | 'equal'
  | 'onlyA' | 'onlyB' | 'onlyC'
  | 'modified' | 'bModified' | 'cModified' | 'bcModified' | 'conflict'

// How two files are declared equal:
//   size+mtime — same size AND same modification time (default, conservative)
//   size       — same size only; faster for large dirs where mtime is unreliable
//   mtime      — same mtime only; useful when sizes are known to match
export type DirCompareMode = 'size+mtime' | 'size' | 'mtime'

export interface DirEntry {
  path: string        // relative path
  isDir: boolean
  status: EntryStatus
  sizeA?: number
  sizeB?: number
  sizeC?: number
  mtimeA?: number
  mtimeB?: number
  mtimeC?: number
}

export interface DirDiffResult {
  entries: DirEntry[]
  baseA: string
  baseB: string
  baseC?: string
}

export interface FsEntry {
  path: string
  isDir: boolean
  size: number
  mtime: number
}

// Called by renderer with data from main process
export function buildDirDiff(
  entriesA: FsEntry[],
  entriesB: FsEntry[],
  entriesC: FsEntry[] | undefined,
  baseA: string,
  baseB: string,
  baseC?: string,
  mode: DirCompareMode = 'size+mtime'
): DirDiffResult {
  const mapA = new Map(entriesA.map(e => [e.path, e]))
  const mapB = new Map(entriesB.map(e => [e.path, e]))
  const mapC = entriesC ? new Map(entriesC.map(e => [e.path, e])) : undefined

  // Core equality predicate — directories are always compared by child rollup,
  // so skip the metadata check for them (scanner records size=0, mtime=0 for dirs).
  const filesEq = (x: FsEntry, y: FsEntry): boolean => {
    if (x.isDir) return true
    if (x.size !== y.size) return false          // size differs → definitely modified
    if (mode === 'size')    return true           // size match is sufficient
    if (mode === 'mtime')   return x.mtime === y.mtime  // size already matched above
    return x.mtime === y.mtime                   // size+mtime: both must match
  }

  const allPaths = new Set([
    ...mapA.keys(),
    ...mapB.keys(),
    ...(mapC ? mapC.keys() : [])
  ])

  const entries: DirEntry[] = []

  for (const p of [...allPaths].sort()) {
    const a = mapA.get(p), b = mapB.get(p), c = mapC?.get(p)

    const abEq = a && b ? filesEq(a, b) : false
    const acEq = a && c ? filesEq(a, c) : false
    const bcEq = b && c ? filesEq(b, c) : false

    let status: EntryStatus
    const has = (x: unknown) => x !== undefined
    if (!mapC) {
      // 2-way
      if (has(a) && !has(b)) status = 'onlyA'
      else if (!has(a) && has(b)) status = 'onlyB'
      else status = abEq ? 'equal' : 'modified'
    } else {
      // 3-way
      if (has(a) && !has(b) && !has(c)) status = 'onlyA'
      else if (!has(a) && has(b) && !has(c)) status = 'onlyB'
      else if (!has(a) && !has(b) && has(c)) status = 'onlyC'
      else if (has(a) && has(b) && !has(c)) status = abEq ? 'equal' : 'modified'
      else if (has(a) && !has(b) && has(c)) status = acEq ? 'equal' : 'cModified'
      else if (!has(a) && has(b) && has(c)) status = bcEq ? 'onlyB' : 'conflict'
      else {
        // all three exist
        if (abEq && acEq) status = 'equal'
        else if (abEq && !acEq) status = 'cModified'
        else if (!abEq && acEq) status = 'bModified'
        else if (bcEq) status = 'bcModified'
        else status = 'conflict'
      }
    }

    entries.push({
      path: p,
      isDir: (a ?? b ?? c)!.isDir,
      status,
      sizeA: a?.size, sizeB: b?.size, sizeC: c?.size,
      mtimeA: a?.mtime, mtimeB: b?.mtime, mtimeC: c?.mtime
    })
  }

  return { entries, baseA, baseB, baseC }
}

export function statusLabel(s: EntryStatus): string {
  return ({
    equal: '=', onlyA: '←A', onlyB: 'B→', onlyC: 'C↓',
    modified: '≠', bModified: 'B~', cModified: 'C~',
    bcModified: 'BC~', conflict: '!!'
  })[s]
}

export function statusColor(s: EntryStatus): string {
  return ({
    equal: 'transparent',
    onlyA: 'var(--col-del-bg)',
    onlyB: 'var(--col-ins-bg)',
    onlyC: 'var(--col-c-bg)',
    modified: 'var(--col-chg-bg)',
    bModified: 'var(--col-chg-bg)',
    cModified: 'var(--col-chg-bg)',
    bcModified: 'var(--col-chg-bg)',
    conflict: 'var(--col-conflict-bg)'
  })[s]
}

export function fmtSize(bytes?: number): string {
  if (bytes === undefined) return ''
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function fmtTime(ms?: number): string {
  if (!ms) return ''
  const d = new Date(ms)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}
