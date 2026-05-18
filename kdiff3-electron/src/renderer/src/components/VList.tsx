import { useRef, useState, useEffect, useCallback, Fragment, ReactNode } from 'react'

const OVERSCAN = 4

interface VListProps<T> {
  items: T[]
  rowHeight: number
  renderItem: (item: T, index: number) => ReactNode
  scrollRef: (el: HTMLDivElement | null) => void
  onScroll: (scrollTop: number) => void
  className?: string
}

// Virtualized fixed-height list. Renders only the visible window + OVERSCAN rows.
// Uses block-flow spacers so horizontal scroll works naturally (no absolute positioning).
export function VList<T>({ items, rowHeight, renderItem, scrollRef, onScroll, className }: VListProps<T>) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const [height, setHeight] = useState(600)
  const [scrollTop, setScrollTop] = useState(0)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    setHeight(el.clientHeight)
    const ro = new ResizeObserver(() => setHeight(el.clientHeight))
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const setRef = useCallback((el: HTMLDivElement | null) => {
    containerRef.current = el
    scrollRef(el)
  }, [scrollRef])

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const top = e.currentTarget.scrollTop
    setScrollTop(top)
    onScroll(top)
  }, [onScroll])

  const len = items.length
  const startIdx = Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN)
  const endIdx   = Math.min(len - 1, Math.ceil((scrollTop + height) / rowHeight) + OVERSCAN)

  const topPad    = startIdx * rowHeight
  const bottomPad = Math.max(0, (len - 1 - endIdx) * rowHeight)

  return (
    <div ref={setRef} className={className} style={{ overflow: 'auto', height: '100%' }} onScroll={handleScroll}>
      {/* min-width: max-content lets content wider than container trigger horizontal scroll */}
      <div style={{ minWidth: 'max-content' }}>
        {topPad > 0 && <div style={{ height: topPad }} />}
        {endIdx >= startIdx && Array.from({ length: endIdx - startIdx + 1 }, (_, i) => {
          const idx = startIdx + i
          return <Fragment key={idx}>{renderItem(items[idx], idx)}</Fragment>
        })}
        {bottomPad > 0 && <div style={{ height: bottomPad }} />}
      </div>
    </div>
  )
}
