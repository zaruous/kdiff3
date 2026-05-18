import { DirDiffResult } from './dirDiff'
import { fmtSize, fmtTime, statusLabel } from './dirDiff'

const STATUS_COLORS: Record<string, { bg: string; color: string }> = {
  equal:      { bg: '#f0f0f0', color: '#666' },
  onlyA:      { bg: '#ffe0e0', color: '#c00' },
  onlyB:      { bg: '#e0ffe0', color: '#060' },
  onlyC:      { bg: '#e0e0ff', color: '#006' },
  modified:   { bg: '#fff8cc', color: '#660' },
  bModified:  { bg: '#fff8cc', color: '#660' },
  cModified:  { bg: '#fff8cc', color: '#660' },
  bcModified: { bg: '#fff8cc', color: '#660' },
  conflict:   { bg: '#ffd0a0', color: '#c40' },
}

export function generateHtmlReport(result: DirDiffResult): string {
  const now = new Date().toLocaleString()
  const entries = result.entries
  const stats = {
    total: entries.length,
    equal: entries.filter(e => e.status === 'equal').length,
    onlyA: entries.filter(e => e.status === 'onlyA').length,
    onlyB: entries.filter(e => e.status === 'onlyB').length,
    modified: entries.filter(e => ['modified','bModified','cModified','bcModified'].includes(e.status)).length,
    conflicts: entries.filter(e => e.status === 'conflict').length,
  }

  const is3way = !!result.baseC

  const rows = entries
    .sort((a, b) => a.path.localeCompare(b.path))
    .map(e => {
      const { bg, color } = STATUS_COLORS[e.status] ?? STATUS_COLORS.equal
      const depth = e.path.split('/').length - 1
      const indent = '&nbsp;'.repeat(depth * 3)
      const icon = e.isDir ? '📁' : '📄'
      const name = e.path.split('/').pop() ?? e.path
      return `
        <tr style="background:${bg}">
          <td style="font-family:monospace; white-space:nowrap">
            ${indent}${icon} ${name}
          </td>
          <td style="color:${color}; font-weight:700; text-align:center; white-space:nowrap">
            ${statusLabel(e.status)}
          </td>
          <td style="color:#666; font-family:monospace; font-size:12px; max-width:280px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap" title="${e.path}">
            ${e.path}
          </td>
          <td style="text-align:right; font-family:monospace; font-size:12px; color:#555">${fmtSize(e.sizeA)}</td>
          <td style="text-align:right; font-family:monospace; font-size:12px; color:#555">${fmtSize(e.sizeB)}</td>
          ${is3way ? `<td style="text-align:right; font-family:monospace; font-size:12px; color:#555">${fmtSize(e.sizeC)}</td>` : ''}
          <td style="font-family:monospace; font-size:11px; color:#888">${fmtTime(e.mtimeA)}</td>
        </tr>`
    }).join('\n')

  const statBar = [
    stats.equal    && `<span style="background:#ddd;color:#555">= 동일 ${stats.equal}</span>`,
    stats.onlyA    && `<span style="background:#fcc;color:#900">← A만 ${stats.onlyA}</span>`,
    stats.onlyB    && `<span style="background:#cfc;color:#060">B만 → ${stats.onlyB}</span>`,
    stats.modified && `<span style="background:#ffc;color:#660">≠ 변경 ${stats.modified}</span>`,
    stats.conflicts && `<span style="background:#f90;color:#630">!! 충돌 ${stats.conflicts}</span>`,
  ].filter(Boolean).join(' &nbsp; ')

  return `<!DOCTYPE html>
<html lang="ko">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>KDiff3 디렉토리 비교 보고서</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0 }
  body { font-family: system-ui, sans-serif; font-size: 13px; background: #fafafa; color: #222 }
  header { background: #1e1e2e; color: #cdd6f4; padding: 20px 24px }
  header h1 { font-size: 20px; margin-bottom: 10px }
  .meta { font-size: 12px; color: #a6adc8; display: flex; flex-wrap: wrap; gap: 16px; margin-bottom: 12px }
  .stat-bar { display: flex; flex-wrap: wrap; gap: 8px }
  .stat-bar span { padding: 3px 10px; border-radius: 12px; font-size: 12px; font-weight: 600 }
  .legend { display: flex; flex-wrap: wrap; gap: 10px; padding: 12px 24px; background: #fff; border-bottom: 1px solid #e0e0e0 }
  .legend-item { display: flex; align-items: center; gap: 6px; font-size: 12px }
  .legend-dot { width: 12px; height: 12px; border-radius: 3px }
  table { width: 100%; border-collapse: collapse; font-size: 13px }
  th { position: sticky; top: 0; background: #2a2a3a; color: #cdd6f4; padding: 8px 10px; text-align: left; font-size: 11px; font-weight: 600; white-space: nowrap }
  td { padding: 4px 10px; border-bottom: 1px solid rgba(0,0,0,0.04) }
  tr:hover td { filter: brightness(0.96) }
  .summary { padding: 16px 24px; background: #fff; border-bottom: 1px solid #e0e0e0; color: #444; font-size: 12px }
  @media print { header { background: #333 !important; color: #fff !important } }
</style>
</head>
<body>
<header>
  <h1>📁 KDiff3 디렉토리 비교 보고서</h1>
  <div class="meta">
    <span>📂 A: <strong>${result.baseA}</strong></span>
    <span>📂 B: <strong>${result.baseB}</strong></span>
    ${result.baseC ? `<span>📂 C: <strong>${result.baseC}</strong></span>` : ''}
    <span>🕐 ${now}</span>
  </div>
  <div class="stat-bar">${statBar}</div>
</header>

<div class="summary">
  전체 <strong>${stats.total}</strong>개 항목 &nbsp;·&nbsp;
  변경 <strong>${stats.total - stats.equal}</strong>개 &nbsp;·&nbsp;
  동일 <strong>${stats.equal}</strong>개
  ${stats.conflicts ? `&nbsp;·&nbsp; <span style="color:#c40; font-weight:700">충돌 ${stats.conflicts}개</span>` : ''}
</div>

<table>
  <thead>
    <tr>
      <th>이름</th>
      <th>상태</th>
      <th>경로</th>
      <th>크기 A</th>
      <th>크기 B</th>
      ${is3way ? '<th>크기 C</th>' : ''}
      <th>수정일 A</th>
    </tr>
  </thead>
  <tbody>
    ${rows}
  </tbody>
</table>
</body>
</html>`
}
