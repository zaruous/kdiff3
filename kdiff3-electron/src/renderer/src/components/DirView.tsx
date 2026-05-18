import { useState, useMemo, useRef, useCallback, useEffect } from 'react'
import { DirDiffResult, EntryStatus, statusColor, fmtSize, fmtTime } from '../lib/dirDiff'
import { buildTree, flattenTree, TreeRow } from '../lib/treeBuilder'
import { generateHtmlReport } from '../lib/htmlReport'
import { VList } from './VList'

const TREE_ROW_H = 22  // must match .tree-row height in CSS

interface Props {
  result: DirDiffResult
  onOpenFile: (pathA: string, pathB: string, pathC?: string) => void
  onRefresh: () => void
}

// ── Status badge ──────────────────────────────────────────────────────────────

function StatusBadge({ status }: { status: EntryStatus }) {
  const label: Record<EntryStatus, string> = {
    equal: '=', onlyA: '←A', onlyB: 'B→', onlyC: 'C↓',
    modified: '≠', bModified: 'B~', cModified: 'C~', bcModified: 'BC~', conflict: '!!'
  }
  return (
    <span
      className="status-chip"
      style={{ background: statusColor(status), color: status === 'equal' ? 'var(--text-dim)' : '#fff', fontSize: 10 }}
    >
      {label[status]}
    </span>
  )
}

// ── Single tree panel ─────────────────────────────────────────────────────────

interface PanelProps {
  side: 'A' | 'B'
  baseDir: string
  rows: TreeRow[]
  selectedPath: string | null
  expandedSet: Set<string>
  onSelect: (path: string, isDir: boolean) => void
  onToggleExpand: (path: string) => void
  onOpenFile: (path: string) => void
  scrollRef: (el: HTMLDivElement | null) => void
  onScroll: (scrollTop: number) => void
}

function TreePanel({ side, baseDir, rows, selectedPath, expandedSet, onSelect, onToggleExpand, onOpenFile, scrollRef, onScroll }: PanelProps) {
  const hasThis = side === 'A' ? (r: TreeRow) => r.hasInA : (r: TreeRow) => r.hasInB

  return (
    <div className="tree-panel-wrap">
      <div className="tree-panel-header">
        <span style={{ fontWeight: 600, color: side === 'A' ? '#f28b82' : '#81c995' }}>
          {side === 'A' ? '◀ A' : 'B ▶'}
        </span>
        <span style={{ color: 'var(--text-dim)', fontSize: 11, marginLeft: 6, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {baseDir}
        </span>
      </div>

      <VList
        items={rows}
        rowHeight={TREE_ROW_H}
        className="tree-panel"
        scrollRef={scrollRef}
        onScroll={onScroll}
        renderItem={(row) => {
          const exists = hasThis(row)
          const isSelected = row.path === selectedPath
          const isExpanded = expandedSet.has(row.path)

          return (
            <div
              key={row.path}
              className={`tree-row${isSelected ? ' selected' : ''}${!exists ? ' ghost' : ''}`}
              style={{ paddingLeft: 8 + row.depth * 16 }}
              onClick={() => {
                if (!exists) return
                if (row.isDir) onToggleExpand(row.path)
                else { onSelect(row.path, false); onOpenFile(row.path) }
              }}
              onContextMenu={() => onSelect(row.path, row.isDir)}
            >
              <span className="tree-expand" onClick={e => { e.stopPropagation(); onToggleExpand(row.path) }}>
                {row.isDir ? (isExpanded ? '▼' : '▶') : ' '}
              </span>
              <span className="tree-icon">{row.isDir ? '📁' : '📄'}</span>
              <span className="tree-name" style={{ color: exists ? 'var(--text)' : 'var(--text-dim)' }}>
                {row.name}
              </span>
              {row.isDir && row.childrenChanged > 0 && exists && (
                <span className="tree-count">{row.childrenChanged}</span>
              )}
              {!row.isDir && exists && (
                <span className="tree-size">
                  {fmtSize(side === 'A' ? row.sizeA : row.sizeB)}
                </span>
              )}
              <StatusBadge status={row.status} />
            </div>
          )
        }}
      />
    </div>
  )
}

// ── Main DirView ──────────────────────────────────────────────────────────────

export default function DirView({ result, onOpenFile, onRefresh }: Props) {
  const [expandedSet, setExpandedSet] = useState<Set<string>>(new Set())
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const [selectedIsDir, setSelectedIsDir] = useState(false)
  const [filter, setFilter] = useState('')
  const [showEqual, setShowEqual] = useState(false)
  const [copyStatus, setCopyStatus] = useState('')

  const is3way = !!result.baseC
  const tree = useMemo(() => buildTree(result.entries), [result.entries])
  const rows = useMemo(
    () => flattenTree(tree, expandedSet, filter, showEqual),
    [tree, expandedSet, filter, showEqual]
  )

  // Expand top-level dirs by default on first load
  useEffect(() => {
    const topDirs = result.entries.filter(e => e.isDir && !e.path.includes('/')).map(e => e.path)
    if (topDirs.length > 0 && expandedSet.size === 0) {
      setExpandedSet(new Set(topDirs))
    }
  }, [result.entries])

  // ── Synchronized scroll ───────────────────────────────────────────────────

  const leftRef  = useRef<HTMLDivElement | null>(null)
  const rightRef = useRef<HTMLDivElement | null>(null)
  const syncing  = useRef(false)

  const onLeftScroll = useCallback((scrollTop: number) => {
    if (syncing.current) return
    const dst = rightRef.current
    if (!dst || Math.abs(dst.scrollTop - scrollTop) <= 0.5) return
    syncing.current = true
    dst.scrollTop = scrollTop
    syncing.current = false
  }, [])

  const onRightScroll = useCallback((scrollTop: number) => {
    if (syncing.current) return
    const dst = leftRef.current
    if (!dst || Math.abs(dst.scrollTop - scrollTop) <= 0.5) return
    syncing.current = true
    dst.scrollTop = scrollTop
    syncing.current = false
  }, [])

  // ── Actions ───────────────────────────────────────────────────────────────

  const toggleExpand = useCallback((path: string) => {
    setExpandedSet(prev => {
      const next = new Set(prev)
      next.has(path) ? next.delete(path) : next.add(path)
      return next
    })
  }, [])

  const expandAll = () => {
    setExpandedSet(new Set(result.entries.filter(e => e.isDir).map(e => e.path)))
  }
  const collapseAll = () => setExpandedSet(new Set())

  const select = (path: string, isDir: boolean) => {
    setSelectedPath(path)
    setSelectedIsDir(isDir)
    setCopyStatus('')
  }

  const openSelectedFile = (path: string) => {
    const e = result.entries.find(x => x.path === path)
    if (!e || e.isDir) return
    const join = (base: string) => `${base}/${path}`
    const pa = e.hasInA ? join(result.baseA) : ''
    const pb = e.hasInB ? join(result.baseB) : ''
    const pc = is3way && e.sizeC !== undefined ? join(result.baseC!) : undefined
    onOpenFile(pa, pb, pc)
  }

  const doCopy = async (srcBase: string, dstBase: string, label: string) => {
    if (!selectedPath) return
    const src = `${srcBase}/${selectedPath}`
    const dst = `${dstBase}/${selectedPath}`
    setCopyStatus('복사 중…')
    const res = await window.api.copyPath(src, dst)
    if (res.ok) {
      setCopyStatus(`✓ ${label} 완료`)
      onRefresh()
    } else {
      setCopyStatus(`✗ ${res.error}`)
    }
  }

  const exportHtml = async () => {
    const html = generateHtmlReport(result)
    const path = await window.api.saveFileDialog()
    if (!path) return
    const savePath = path.endsWith('.html') ? path : path + '.html'
    const res = await window.api.writeFile(savePath, html)
    setCopyStatus(res.ok ? `✓ 보고서 저장: ${savePath}` : `✗ ${res.error}`)
  }

  // Which side is the selected entry on?
  const selEntry = selectedPath ? result.entries.find(e => e.path === selectedPath) : null
  const canCopyAtoB = selEntry && selEntry.hasInA
  const canCopyBtoA = selEntry && selEntry.hasInB

  const stats = useMemo(() => ({
    total: result.entries.length,
    changed: result.entries.filter(e => e.status !== 'equal').length,
    conflicts: result.entries.filter(e => e.status === 'conflict').length,
  }), [result.entries])

  return (
    <div className="dir-view">
      {/* ── Toolbar ── */}
      <div className="dir-view-toolbar">
        <div className="dir-toolbar-left">
          <input
            className="filter-input"
            type="search"
            placeholder="🔍 경로 필터"
            value={filter}
            onChange={e => setFilter(e.target.value)}
          />
          <label className="check-label">
            <input type="checkbox" checked={showEqual} onChange={e => setShowEqual(e.target.checked)} />
            동일 표시
          </label>
          <button onClick={expandAll} title="모두 펼치기">⊞ 펼치기</button>
          <button onClick={collapseAll} title="모두 접기">⊟ 접기</button>
          <div className="toolbar-sep" />
          <span className="stat-text">전체 {stats.total}</span>
          {stats.changed > 0 && <span className="stat-text changed">변경 {stats.changed}</span>}
          {stats.conflicts > 0 && <span className="stat-text conflict">충돌 {stats.conflicts}</span>}
        </div>

        <div className="dir-toolbar-right">
          {/* Copy action bar */}
          {selectedPath && (
            <div className="copy-bar">
              <button
                className="copy-btn copy-to-a"
                disabled={!canCopyBtoA}
                onClick={() => doCopy(result.baseB, result.baseA, 'B→A 복사')}
                title="선택 항목을 B에서 A로 복사"
              >
                ← A로 복사
              </button>
              <span className="copy-selected" title={selectedPath}>
                {selectedIsDir ? '📁' : '📄'} {selectedPath.split('/').pop()}
              </span>
              <button
                className="copy-btn copy-to-b"
                disabled={!canCopyAtoB}
                onClick={() => doCopy(result.baseA, result.baseB, 'A→B 복사')}
                title="선택 항목을 A에서 B로 복사"
              >
                B로 복사 →
              </button>
              <button
                className="deselect-btn"
                onClick={() => { setSelectedPath(null); setCopyStatus('') }}
              >
                ✕
              </button>
            </div>
          )}
          {copyStatus && (
            <span className={`copy-status ${copyStatus.startsWith('✓') ? 'ok' : copyStatus.startsWith('✗') ? 'err' : 'pending'}`}>
              {copyStatus}
            </span>
          )}
          <button className="export-btn" onClick={exportHtml} title="HTML 보고서 내보내기">
            📄 HTML 보고서
          </button>
        </div>
      </div>

      {/* ── Two tree panels ── */}
      <div className="tree-split">
        <TreePanel
          side="A"
          baseDir={result.baseA}
          rows={rows}
          selectedPath={selectedPath}
          expandedSet={expandedSet}
          onSelect={select}
          onToggleExpand={toggleExpand}
          onOpenFile={openSelectedFile}
          scrollRef={el => { leftRef.current = el }}
          onScroll={onLeftScroll}
        />

        <div className="tree-divider" />

        <TreePanel
          side="B"
          baseDir={result.baseB}
          rows={rows}
          selectedPath={selectedPath}
          expandedSet={expandedSet}
          onSelect={select}
          onToggleExpand={toggleExpand}
          onOpenFile={openSelectedFile}
          scrollRef={el => { rightRef.current = el }}
          onScroll={onRightScroll}
        />
      </div>

      {/* ── Detail strip ── */}
      {selEntry && (
        <div className="detail-strip">
          <span className="detail-label">선택:</span>
          <span className="detail-path">{selEntry.path}</span>
          {!selEntry.isDir && (
            <>
              <span className="detail-meta">A: {fmtSize(selEntry.sizeA) || '—'} · {fmtTime(selEntry.mtimeA) || '—'}</span>
              <span className="detail-meta">B: {fmtSize(selEntry.sizeB) || '—'} · {fmtTime(selEntry.mtimeB) || '—'}</span>
            </>
          )}
          <StatusBadge status={selEntry.status} />
          {!selEntry.isDir && selEntry.status !== 'equal' && (
            <button className="open-diff-btn" onClick={() => openSelectedFile(selEntry.path)}>
              파일 비교 열기 →
            </button>
          )}
        </div>
      )}
    </div>
  )
}
