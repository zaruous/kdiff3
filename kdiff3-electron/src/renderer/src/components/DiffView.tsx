import { useRef, useCallback } from 'react'
import { DiffLine, computeWordDiff, WordToken } from '../lib/diff'

interface Props {
  lines: DiffLine[]
  pathA: string
  pathB: string
  pathC?: string
  showLineNumbers: boolean
  is3way?: boolean
}

// Sync scroll between panels without causing loops
function useSyncScroll(count: number) {
  const refs = useRef<(HTMLDivElement | null)[]>(Array(count).fill(null))
  const syncing = useRef(false)

  const setRef = (i: number) => (el: HTMLDivElement | null) => {
    refs.current[i] = el
  }

  const onScroll = useCallback((srcIdx: number) => () => {
    if (syncing.current) return
    const src = refs.current[srcIdx]
    if (!src) return
    syncing.current = true
    refs.current.forEach((el, i) => {
      if (i !== srcIdx && el) {
        el.scrollTop = src.scrollTop
        el.scrollLeft = src.scrollLeft
      }
    })
    syncing.current = false
  }, [])

  return { setRef, onScroll }
}

// Render a line with word-level highlighting for 'changed' lines
function WordHighlight({ tokens, side }: { tokens: WordToken[]; side: 'a' | 'b' }) {
  return (
    <>
      {tokens.map((tok, i) => {
        if (tok.type === 'equal') return <span key={i}>{tok.value}</span>
        const cls = side === 'a' ? 'word-chg-a' : 'word-chg-b'
        return <span key={i} className={cls}>{tok.value}</span>
      })}
    </>
  )
}

function DiffLineRow({
  line, panelIdx, showLineNumbers
}: {
  line: DiffLine
  panelIdx: 0 | 1 | 2
  showLineNumbers: boolean
}) {
  const isA = panelIdx === 0
  const isB = panelIdx === 1
  const isC = panelIdx === 2

  const text = isA ? line.textA : isB ? line.textB : line.textA  // C uses textA slot in 3-way
  const lineNum = isA ? line.numA : isB ? line.numB : undefined

  const cls =
    line.type === 'delete' ? (isA ? 'del' : 'empty') :
    line.type === 'insert' ? (isB ? 'ins' : 'empty') :
    line.type === 'changed' ? 'chg' : ''

  const wordDiff = line.type === 'changed' ? computeWordDiff(line.textA, line.textB) : null

  return (
    <div className={`diff-line ${cls}`}>
      {showLineNumbers && (
        <span className="diff-line-num">{lineNum ?? ''}</span>
      )}
      <span className="diff-line-content">
        {wordDiff ? (
          <WordHighlight tokens={isA ? wordDiff.a : wordDiff.b} side={isA ? 'a' : 'b'} />
        ) : (
          text
        )}
      </span>
    </div>
  )
}

export default function DiffView({ lines, pathA, pathB, pathC, showLineNumbers, is3way }: Props) {
  const panelCount = is3way ? 3 : 2
  const { setRef, onScroll } = useSyncScroll(panelCount)

  const headers = [pathA || 'File A (base)', pathB || 'File B', pathC || 'File C']

  return (
    <div className="diff-view">
      <div className={`diff-header${is3way ? ' threeway' : ''}`}>
        {headers.slice(0, panelCount).map((h, i) => (
          <div key={i} className="diff-panel-header" title={h}>{h}</div>
        ))}
      </div>

      <div className={`diff-panels${is3way ? ' threeway' : ''}`}>
        {Array.from({ length: panelCount }, (_, pi) => (
          <div
            key={pi}
            className="diff-panel"
            ref={setRef(pi)}
            onScroll={onScroll(pi)}
          >
            {lines.map((line, li) => (
              <DiffLineRow
                key={li}
                line={line}
                panelIdx={pi as 0 | 1 | 2}
                showLineNumbers={showLineNumbers}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  )
}
