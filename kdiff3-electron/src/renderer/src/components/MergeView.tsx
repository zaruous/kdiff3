import { useRef, useEffect } from 'react'
import {
  MergeResult, MergeBlock, ResolvedChoice,
  resolveBlock, generateOutput, nextConflict, prevConflict
} from '../lib/merger'

interface Props {
  result: MergeResult
  onChange: (r: MergeResult) => void
  currentConflict: number
  onCurrentConflict: (i: number) => void
}

function MergeLineRow({ text, lineNum }: { text: string; lineNum?: number }) {
  return (
    <div className="merge-line">
      <span className="line-num">{lineNum ?? ''}</span>
      <span>{text}</span>
    </div>
  )
}

function ConflictBlock({
  block, idx, isCurrent, onClick, onResolve
}: {
  block: MergeBlock
  idx: number
  isCurrent: boolean
  onClick: () => void
  onResolve: (choice: ResolvedChoice) => void
}) {
  const blockRef = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (isCurrent) blockRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
  }, [isCurrent])

  const resolvedLabel = block.resolved
    ? `해결됨 → ${block.resolved}`
    : '★ 충돌'

  return (
    <div className="merge-block conflict-block" ref={blockRef}>
      <div className={`conflict-header${isCurrent ? ' current' : ''}`} onClick={onClick}>
        <span>충돌 #{idx + 1}  {resolvedLabel}</span>
        {!block.resolved && (
          <span style={{ color: '#ffaaaa', fontSize: 11 }}>클릭하여 선택</span>
        )}
      </div>

      <div className="conflict-section a">
        <div className="conflict-section-label">◀ A (base)</div>
        {block.linesA.map((l, i) => (
          <MergeLineRow key={i} text={l} lineNum={block.startA + i} />
        ))}
      </div>
      <div className="conflict-section b">
        <div className="conflict-section-label">▶ B</div>
        {block.linesB.map((l, i) => (
          <MergeLineRow key={i} text={l} lineNum={block.startB + i} />
        ))}
      </div>
      {block.linesC.length > 0 && (
        <div className="conflict-section c">
          <div className="conflict-section-label">▼ C</div>
          {block.linesC.map((l, i) => (
            <MergeLineRow key={i} text={l} lineNum={block.startC + i} />
          ))}
        </div>
      )}

      <div className="resolve-bar">
        {block.resolved ? (
          <span className="resolve-bar" style={{ fontSize: 11, color: '#6ddf6d', padding: 0 }}>
            ✓ {block.resolved} 선택됨
          </span>
        ) : (
          <>
            <button onClick={() => onResolve('A')}>A 선택</button>
            <button onClick={() => onResolve('B')}>B 선택</button>
            {block.linesC.length > 0 && (
              <button onClick={() => onResolve('C')}>C 선택</button>
            )}
          </>
        )}
      </div>
    </div>
  )
}

function AutoBlock({ block, detail }: { block: MergeBlock; detail: string }) {
  const cls =
    detail.startsWith('B') ? 'b-change' :
    detail.startsWith('C') ? 'c-change' :
    'bc-equal'
  const lines = detail.startsWith('C') ? block.linesC : block.linesB
  const startNum = detail.startsWith('C') ? block.startC : block.startB

  return (
    <div className={`merge-block auto-block ${cls}`}>
      {lines.map((l, i) => (
        <div key={i} className="merge-line">
          <span className="line-num">{startNum + i}</span>
          <span>{l}</span>
        </div>
      ))}
    </div>
  )
}

export default function MergeView({ result, onChange, currentConflict, onCurrentConflict }: Props) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const savePathRef = useRef<HTMLInputElement>(null)
  const saveStatusRef = useRef<HTMLSpanElement>(null)

  const resolve = (idx: number, choice: ResolvedChoice) => {
    onChange(resolveBlock(result, idx, choice))
    onCurrentConflict(idx)
  }

  const goNext = () => {
    const i = nextConflict(result, currentConflict)
    if (i >= 0) onCurrentConflict(i)
  }
  const goPrev = () => {
    const i = prevConflict(result, currentConflict)
    if (i >= 0) onCurrentConflict(i)
  }

  const doSave = async () => {
    const path = savePathRef.current?.value.trim()
    if (!path) {
      if (saveStatusRef.current) saveStatusRef.current.textContent = '경로를 입력하세요'
      return
    }
    const content = generateOutput(result)
    const res = await window.api.writeFile(path, content)
    if (saveStatusRef.current)
      saveStatusRef.current.textContent = res.ok ? '✓ 저장 완료' : `✗ ${res.error}`
  }

  const pickSavePath = async () => {
    const p = await window.api.saveFileDialog()
    if (p && savePathRef.current) savePathRef.current.value = p
  }

  return (
    <div className="merge-view">
      {/* Toolbar */}
      <div className="merge-toolbar">
        <span className={`conflict-badge ${result.unresolvedCount > 0 ? 'has-conflicts' : 'resolved'}`}>
          충돌 {result.conflictCount}개 · 미해결 {result.unresolvedCount}개
        </span>
        <div className="sep" style={{ width: 1, height: 18, background: 'var(--border)', flexShrink: 0 }} />
        <button onClick={goPrev} disabled={result.conflictCount === 0}>◀ 이전</button>
        <button onClick={goNext} disabled={result.conflictCount === 0}>다음 ▶</button>
        <div className="sep" style={{ width: 1, height: 18, background: 'var(--border)', flexShrink: 0 }} />
        <span style={{ fontSize: 11, color: 'var(--text-dim)' }}>
          Ctrl+↑↓ 이동 · Ctrl+1/2/3 해결 · Ctrl+S 저장
        </span>
      </div>

      {/* Save row */}
      <div className="save-row">
        <span style={{ fontSize: 11, color: 'var(--text-dim)', flexShrink: 0 }}>출력:</span>
        <input type="text" ref={savePathRef} placeholder="/path/to/output.txt" />
        <button onClick={pickSavePath}>…</button>
        <button className="primary" onClick={doSave}>저장</button>
        <span className="save-status ok" ref={saveStatusRef} />
      </div>

      {/* Merge blocks */}
      <div className="merge-scroll" ref={scrollRef}>
        {result.blocks.map((block, idx) => {
          if (block.detail === 'NoChange') {
            const lines = block.linesA
            if (lines.length > 6) {
              return (
                <div key={idx} className="merge-block no-change">
                  <div className="merge-line collapsed">  ... {lines.length}줄 동일 ...</div>
                </div>
              )
            }
            return (
              <div key={idx} className="merge-block no-change">
                {lines.map((l, i) => (
                  <div key={i} className="merge-line">
                    <span className="line-num">{block.startA + i}</span>
                    <span>{l}</span>
                  </div>
                ))}
              </div>
            )
          }

          if (block.detail === 'Conflict') {
            return (
              <ConflictBlock
                key={idx}
                block={block}
                idx={idx}
                isCurrent={idx === currentConflict}
                onClick={() => onCurrentConflict(idx)}
                onResolve={(c) => resolve(idx, c)}
              />
            )
          }

          return <AutoBlock key={idx} block={block} detail={block.detail} />
        })}
      </div>
    </div>
  )
}
