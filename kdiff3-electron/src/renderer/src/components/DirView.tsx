import { useState, useMemo } from 'react'
import {
  DirDiffResult, DirEntry, EntryStatus,
  statusLabel, statusColor, fmtSize, fmtTime
} from '../lib/dirDiff'

type SortKey = 'path' | 'status' | 'sizeA' | 'sizeB' | 'mtimeA'

interface Props {
  result: DirDiffResult
  onOpenFile: (pathA: string, pathB: string, pathC?: string) => void
}

function StatusChip({ status }: { status: EntryStatus }) {
  const style = {
    background: statusColor(status),
    color: status === 'equal' ? 'var(--text-dim)' : '#fff'
  }
  return <span className="status-chip" style={style}>{statusLabel(status)}</span>
}

export default function DirView({ result, onOpenFile }: Props) {
  const [filter, setFilter] = useState('')
  const [showEqual, setShowEqual] = useState(false)
  const [sortKey, setSortKey] = useState<SortKey>('path')
  const [sortAsc, setSortAsc] = useState(true)

  const is3way = !!result.baseC

  const sorted = useMemo(() => {
    let list = result.entries
    if (!showEqual) list = list.filter(e => e.status !== 'equal')
    if (filter) {
      const f = filter.toLowerCase()
      list = list.filter(e => e.path.toLowerCase().includes(f))
    }
    const cmp = (a: DirEntry, b: DirEntry): number => {
      let v = 0
      if (sortKey === 'path')   v = a.path.localeCompare(b.path)
      if (sortKey === 'status') v = a.status.localeCompare(b.status)
      if (sortKey === 'sizeA')  v = (a.sizeA ?? 0) - (b.sizeA ?? 0)
      if (sortKey === 'sizeB')  v = (a.sizeB ?? 0) - (b.sizeB ?? 0)
      if (sortKey === 'mtimeA') v = (a.mtimeA ?? 0) - (b.mtimeA ?? 0)
      return sortAsc ? v : -v
    }
    return [...list].sort(cmp)
  }, [result.entries, filter, showEqual, sortKey, sortAsc])

  const stats = useMemo(() => ({
    total: result.entries.length,
    changed: result.entries.filter(e => e.status !== 'equal').length,
    conflicts: result.entries.filter(e => e.status === 'conflict').length
  }), [result.entries])

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) setSortAsc(a => !a)
    else { setSortKey(key); setSortAsc(true) }
  }

  const sortArrow = (key: SortKey) =>
    sortKey === key ? (sortAsc ? ' ▲' : ' ▼') : ''

  const handleRowClick = (entry: DirEntry) => {
    if (entry.isDir) return
    const join = (base: string, rel: string) => `${base}/${rel}`
    const pa = entry.sizeA !== undefined ? join(result.baseA, entry.path) : ''
    const pb = entry.sizeB !== undefined ? join(result.baseB, entry.path) : ''
    const pc = result.baseC && entry.sizeC !== undefined ? join(result.baseC, entry.path) : undefined
    onOpenFile(pa, pb, pc)
  }

  return (
    <div className="dir-view">
      <div className="dir-toolbar">
        <label>
          <input type="checkbox" checked={showEqual} onChange={e => setShowEqual(e.target.checked)} />
          동일 파일 표시
        </label>
        <span className="sep" style={{ width: 1, height: 18, background: 'var(--border)', flexShrink: 0 }} />
        <input
          type="search"
          value={filter}
          onChange={e => setFilter(e.target.value)}
          placeholder="🔍 경로 필터"
          style={{ width: 180 }}
        />
        <span className="sep" style={{ width: 1, height: 18, background: 'var(--border)', flexShrink: 0 }} />
        <span style={{ fontSize: 11, color: 'var(--text-dim)' }}>전체 {stats.total}</span>
        <span style={{ fontSize: 11, color: stats.changed > 0 ? '#e8c46a' : 'var(--text-dim)' }}>
          변경 {stats.changed}
        </span>
        {stats.conflicts > 0 && (
          <span style={{ fontSize: 11, color: '#ff7070' }}>충돌 {stats.conflicts}</span>
        )}
      </div>

      <div className="dir-table-wrap">
        <table className="dir-table">
          <thead>
            <tr>
              <th className={sortKey === 'status' ? 'sorted' : ''} onClick={() => toggleSort('status')} style={{ width: 50 }}>
                상태{sortArrow('status')}
              </th>
              <th className={sortKey === 'path' ? 'sorted' : ''} onClick={() => toggleSort('path')}>
                경로{sortArrow('path')}
              </th>
              <th className={sortKey === 'sizeA' ? 'sorted' : ''} onClick={() => toggleSort('sizeA')} style={{ width: 80 }}>
                크기 A{sortArrow('sizeA')}
              </th>
              <th className={sortKey === 'sizeB' ? 'sorted' : ''} onClick={() => toggleSort('sizeB')} style={{ width: 80 }}>
                크기 B{sortArrow('sizeB')}
              </th>
              {is3way && <th style={{ width: 80 }}>크기 C</th>}
              <th className={sortKey === 'mtimeA' ? 'sorted' : ''} onClick={() => toggleSort('mtimeA')} style={{ width: 130 }}>
                수정일 A{sortArrow('mtimeA')}
              </th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((entry, i) => (
              <tr
                key={i}
                onClick={() => handleRowClick(entry)}
                title={entry.isDir ? entry.path : `${entry.path} — 클릭하면 파일 비교`}
              >
                <td><StatusChip status={entry.status} /></td>
                <td>{entry.isDir ? '📁 ' : '📄 '}{entry.path}</td>
                <td style={{ textAlign: 'right', color: 'var(--text-dim)' }}>{fmtSize(entry.sizeA)}</td>
                <td style={{ textAlign: 'right', color: 'var(--text-dim)' }}>{fmtSize(entry.sizeB)}</td>
                {is3way && <td style={{ textAlign: 'right', color: 'var(--text-dim)' }}>{fmtSize(entry.sizeC)}</td>}
                <td style={{ color: 'var(--text-dim)' }}>{fmtTime(entry.mtimeA)}</td>
              </tr>
            ))}
          </tbody>
        </table>

        {sorted.length === 0 && (
          <div className="empty-state">
            {filter ? '필터 조건에 맞는 항목이 없습니다.' : '변경된 항목이 없습니다.'}
          </div>
        )}
      </div>
    </div>
  )
}
