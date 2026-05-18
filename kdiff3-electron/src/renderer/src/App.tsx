import { useState, useEffect, useCallback, useRef } from 'react'
import { computeLineDiff, computeStats, DiffLine, DiffOptions } from './lib/diff'
import { buildMerger, MergeResult, ResolvedChoice, resolveBlock, nextConflict, prevConflict } from './lib/merger'
import { buildDirDiff, DirDiffResult, DirCompareMode } from './lib/dirDiff'
import DiffView from './components/DiffView'
import MergeView from './components/MergeView'
import DirView from './components/DirView'

type Tab = 'diff' | 'merge' | 'dir'

interface PathState { a: string; b: string; c: string }
interface DirPathState { a: string; b: string; c: string }

export default function App() {
  const [tab, setTab] = useState<Tab>('diff')
  const [paths, setPaths] = useState<PathState>({ a: '', b: '', c: '' })
  const [dirPaths, setDirPaths] = useState<DirPathState>({ a: '', b: '', c: '' })

  const [diffLines, setDiffLines] = useState<DiffLine[]>([])
  const [mergeResult, setMergeResult] = useState<MergeResult | null>(null)
  const [dirResult, setDirResult] = useState<DirDiffResult | null>(null)

  const [currentConflict, setCurrentConflict] = useState(0)
  const [showLineNumbers, setShowLineNumbers] = useState(true)
  const [options, setOptions] = useState<DiffOptions>({ ignoreWhitespace: false, ignoreCase: false })
  const [status, setStatus] = useState('파일 경로를 입력하고 비교 버튼을 누르세요.')
  const [scanProgress, setScanProgress] = useState<{ a: number; b: number; c: number } | null>(null)
  const [dirCompareMode, setDirCompareMode] = useState<DirCompareMode>('size+mtime')
  const activeScanIds = useRef<string[]>([])

  // ── File comparison ─────────────────────────────────────────────────────────

  const loadAndDiff = useCallback(async (p?: PathState, opts?: DiffOptions) => {
    const ps = p ?? paths
    const o  = opts ?? options
    const readLines = async (path: string): Promise<string[] | null> => {
      if (!path.trim()) return null
      const res = await window.api.readFile(path.trim())
      if (!res.ok) { setStatus(`오류: ${res.error}`); return null }
      return res.content.split('\n')
    }

    const [la, lb, lc] = await Promise.all([
      readLines(ps.a), readLines(ps.b), readLines(ps.c)
    ])
    if (!la || !lb) { if (!la && !lb) setStatus('A, B 파일 경로를 입력하세요.'); return }

    const diff = computeLineDiff(la, lb, o)
    setDiffLines(diff)

    const merger = buildMerger(la, lb, lc ?? [])
    setMergeResult(merger)
    setCurrentConflict(0)

    const s = computeStats(diff)
    setStatus(
      `+${s.additions} 추가  -${s.deletions} 삭제  ~${s.changes} 변경` +
      (merger.conflictCount > 0 ? `  ★ 충돌: ${merger.conflictCount}개` : '')
    )
  }, [paths, options])

  // ── Directory comparison ────────────────────────────────────────────────────

  const cancelDirScan = useCallback(() => {
    for (const id of activeScanIds.current) window.api.scanDirCancel(id)
    activeScanIds.current = []
    setScanProgress(null)
  }, [])

  const runDirDiff = useCallback(async (dp?: DirPathState) => {
    const dps = dp ?? dirPaths
    if (!dps.a.trim() || !dps.b.trim()) {
      setStatus('디렉토리 A, B 경로를 입력하세요.')
      return
    }

    cancelDirScan()

    const idA = crypto.randomUUID()
    const idB = crypto.randomUUID()
    const idC = dps.c.trim() ? crypto.randomUUID() : null
    activeScanIds.current = idC ? [idA, idB, idC] : [idA, idB]

    const counts = { a: 0, b: 0, c: 0 }
    setScanProgress({ ...counts })
    setStatus('스캔 중…')

    const cleanup = window.api.onScanProgress((data) => {
      if (data.scanId === idA) counts.a = data.count
      else if (data.scanId === idB) counts.b = data.count
      else if (idC && data.scanId === idC) counts.c = data.count
      setScanProgress({ ...counts })
    })

    try {
      const [ra, rb, rc] = await Promise.all([
        window.api.scanDirStart(idA, dps.a.trim()),
        window.api.scanDirStart(idB, dps.b.trim()),
        idC ? window.api.scanDirStart(idC, dps.c.trim()) : Promise.resolve(null),
      ])

      cleanup()
      activeScanIds.current = []
      setScanProgress(null)

      if (ra.cancelled || rb.cancelled) { setStatus('스캔 취소됨'); return }
      if (!ra.ok) { setStatus(`디렉토리 A 오류: ${ra.error}`); return }
      if (!rb.ok) { setStatus(`디렉토리 B 오류: ${rb.error}`); return }
      if (rc && !rc.ok) { setStatus(`디렉토리 C 오류: ${(rc as {ok:false;error?:string}).error}`); return }

      const ea = ra.entries
      const eb = rb.entries
      const ec = rc && rc.ok ? rc.entries : undefined

      const result = buildDirDiff(ea, eb, ec, dps.a.trim(), dps.b.trim(), dps.c.trim() || undefined, dirCompareMode)
      setDirResult(result)

      const changed = result.entries.filter(e => e.status !== 'equal').length
      const conflicts = result.entries.filter(e => e.status === 'conflict').length
      const modeLabel = dirCompareMode === 'size' ? '크기만' : dirCompareMode === 'mtime' ? '시간만' : '크기+시간'
      setStatus(
        `디렉토리 전체 ${result.entries.length}개  변경 ${changed}개` +
        (conflicts > 0 ? `  ★ 충돌 ${conflicts}개` : '') +
        `  [${modeLabel}]`
      )
    } catch (e) {
      cleanup()
      activeScanIds.current = []
      setScanProgress(null)
      setStatus(`스캔 오류: ${String(e)}`)
    }
  }, [dirPaths, cancelDirScan, dirCompareMode])

  // ── File open from DirView ──────────────────────────────────────────────────

  const handleOpenFile = useCallback((pathA: string, pathB: string, pathC?: string) => {
    const ps = { a: pathA, b: pathB, c: pathC ?? '' }
    setPaths(ps)
    setTab('diff')
    loadAndDiff(ps, options)
  }, [loadAndDiff, options])

  // ── Keyboard shortcuts (Merge tab) ──────────────────────────────────────────

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (!e.ctrlKey && !e.metaKey) return
      if (tab !== 'merge' || !mergeResult) return
      if (e.key === 'ArrowDown') { e.preventDefault(); setCurrentConflict(i => { const n = nextConflict(mergeResult, i); return n >= 0 ? n : i }) }
      if (e.key === 'ArrowUp')   { e.preventDefault(); setCurrentConflict(i => { const n = prevConflict(mergeResult, i); return n >= 0 ? n : i }) }
      if (e.key === '1') { e.preventDefault(); setMergeResult(r => r ? resolveBlock(r, currentConflict, 'A') : r) }
      if (e.key === '2') { e.preventDefault(); setMergeResult(r => r ? resolveBlock(r, currentConflict, 'B') : r) }
      if (e.key === '3') { e.preventDefault(); setMergeResult(r => r ? resolveBlock(r, currentConflict, 'C') : r) }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [tab, mergeResult, currentConflict])

  // ── Path input helpers ──────────────────────────────────────────────────────

  const pickFile = async (key: keyof PathState, title: string) => {
    const p = await window.api.openFileDialog({ title })
    if (p) setPaths(prev => ({ ...prev, [key]: p }))
  }

  const pickDir = async (key: keyof DirPathState, title: string) => {
    const p = await window.api.openDirDialog({ title })
    if (p) setDirPaths(prev => ({ ...prev, [key]: p }))
  }

  const setOpt = (k: keyof DiffOptions, v: boolean) =>
    setOptions(prev => ({ ...prev, [k]: v }))

  // ── Render ──────────────────────────────────────────────────────────────────

  return (
    <div className="app">
      {/* Toolbar */}
      <div className="toolbar">
        {(tab === 'diff' || tab === 'merge') && (
          <div className="toolbar-row">
            <div className="path-group">
              <label>A</label>
              <input type="text" value={paths.a} onChange={e => setPaths(p => ({ ...p, a: e.target.value }))} placeholder="파일 A (base)" />
              <button onClick={() => pickFile('a', 'Open File A')}>…</button>
            </div>
            <div className="path-group">
              <label>B</label>
              <input type="text" value={paths.b} onChange={e => setPaths(p => ({ ...p, b: e.target.value }))} placeholder="파일 B" />
              <button onClick={() => pickFile('b', 'Open File B')}>…</button>
            </div>
            <div className="path-group">
              <label>C</label>
              <input type="text" value={paths.c} onChange={e => setPaths(p => ({ ...p, c: e.target.value }))} placeholder="파일 C (선택)" />
              <button onClick={() => pickFile('c', 'Open File C')}>…</button>
            </div>
            <button className="primary" onClick={() => loadAndDiff()}>비교</button>
            <div className="sep" />
            <label><input type="checkbox" checked={options.ignoreWhitespace} onChange={e => setOpt('ignoreWhitespace', e.target.checked)} /> 공백 무시</label>
            <label><input type="checkbox" checked={options.ignoreCase} onChange={e => setOpt('ignoreCase', e.target.checked)} /> 대소문자 무시</label>
            <label><input type="checkbox" checked={showLineNumbers} onChange={e => setShowLineNumbers(e.target.checked)} /> 줄 번호</label>
          </div>
        )}

        {tab === 'dir' && (
          <div className="toolbar-row">
            <div className="path-group">
              <label>A</label>
              <input type="text" value={dirPaths.a} onChange={e => setDirPaths(p => ({ ...p, a: e.target.value }))} placeholder="디렉토리 A" />
              <button onClick={() => pickDir('a', 'Open Directory A')}>…</button>
            </div>
            <div className="path-group">
              <label>B</label>
              <input type="text" value={dirPaths.b} onChange={e => setDirPaths(p => ({ ...p, b: e.target.value }))} placeholder="디렉토리 B" />
              <button onClick={() => pickDir('b', 'Open Directory B')}>…</button>
            </div>
            <div className="path-group">
              <label>C</label>
              <input type="text" value={dirPaths.c} onChange={e => setDirPaths(p => ({ ...p, c: e.target.value }))} placeholder="선택 (3-way)" />
              <button onClick={() => pickDir('c', 'Open Directory C')}>…</button>
            </div>
            <div className="sep" />
            <label className="compare-mode-label">비교 기준</label>
            <div className="compare-mode-group">
              {(['size+mtime', 'size', 'mtime'] as DirCompareMode[]).map(m => (
                <button
                  key={m}
                  className={`mode-btn${dirCompareMode === m ? ' active' : ''}`}
                  onClick={() => setDirCompareMode(m)}
                  title={m === 'size+mtime' ? '크기와 수정시간 모두 일치 시 동일' : m === 'size' ? '크기만 일치 시 동일 (대용량 권장)' : '수정시간만 일치 시 동일'}
                >
                  {m === 'size+mtime' ? '크기+시간' : m === 'size' ? '크기만' : '시간만'}
                </button>
              ))}
            </div>
            {scanProgress ? (
              <button className="primary cancel" onClick={cancelDirScan}>
                취소 (A:{scanProgress.a} B:{scanProgress.b}{scanProgress.c ? ` C:${scanProgress.c}` : ''})
              </button>
            ) : (
              <button className="primary" onClick={() => runDirDiff()}>폴더 비교</button>
            )}
          </div>
        )}

        {/* Tab bar */}
        <div className="tab-bar">
          <button className={`tab${tab === 'diff'  ? ' active' : ''}`} onClick={() => setTab('diff')}>📄 파일 비교</button>
          <button className={`tab${tab === 'merge' ? ' active' : ''}`} onClick={() => setTab('merge')}>🔀 병합</button>
          <button className={`tab${tab === 'dir'   ? ' active' : ''}`} onClick={() => setTab('dir')}>📁 폴더 비교</button>
        </div>
      </div>

      {/* Content */}
      <div className="content-area">
        {tab === 'diff' && (
          diffLines.length > 0 ? (
            <DiffView
              lines={diffLines}
              pathA={paths.a} pathB={paths.b} pathC={paths.c || undefined}
              showLineNumbers={showLineNumbers}
              is3way={!!paths.c}
            />
          ) : (
            <div className="empty-state">파일 경로를 입력하고 비교 버튼을 누르세요.</div>
          )
        )}

        {tab === 'merge' && (
          mergeResult ? (
            <MergeView
              result={mergeResult}
              onChange={setMergeResult}
              currentConflict={currentConflict}
              onCurrentConflict={setCurrentConflict}
            />
          ) : (
            <div className="empty-state">먼저 파일 비교를 실행하세요.</div>
          )
        )}

        {tab === 'dir' && (
          dirResult ? (
            <DirView result={dirResult} onOpenFile={handleOpenFile} onRefresh={() => runDirDiff()} />
          ) : (
            <div className="empty-state">디렉토리 경로를 입력하고 폴더 비교 버튼을 누르세요.</div>
          )
        )}
      </div>

      {/* Status bar */}
      <div className="status-bar">
        <span>{status}</span>
      </div>
    </div>
  )
}
