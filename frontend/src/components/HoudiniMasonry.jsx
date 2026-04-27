import { useEffect, useMemo, useRef, useState } from 'react'
import './HoudiniMasonry.css'

const cards = [
  { title: 'Validator', subtitle: 'Live mempool verification', accent: 4 },
  { title: 'Bridge', subtitle: 'Cross-chain asset metadata', accent: 8 },
  { title: 'Oracles', subtitle: 'Real-time price feeds', accent: 2 },
  { title: 'Governance', subtitle: 'Policy & voting state', accent: 6 },
  { title: 'Analytics', subtitle: 'Transaction metrics', accent: 10 },
  { title: 'Storage', subtitle: 'Distributed asset index', accent: 3 },
]

function HoudiniMasonry() {
  const [supportsHoudini, setSupportsHoudini] = useState(false)
  const gridRef = useRef(null)

  useEffect(() => {
    const layoutSupport = window.CSS?.layoutWorklet
    const paintSupport = window.CSS?.paintWorklet
    if (layoutSupport && paintSupport) {
      Promise.all([
        CSS.paintWorklet.addModule(new URL('../houdini/card-background-painter.js', import.meta.url)),
        CSS.layoutWorklet.addModule(new URL('../houdini/masonry-layout.js', import.meta.url)),
      ])
        .then(() => setSupportsHoudini(true))
        .catch(() => setSupportsHoudini(false))
      return
    }
    setSupportsHoudini(false)
  }, [])

  useEffect(() => {
    if (supportsHoudini || !gridRef.current) {
      return
    }

    const resizeObserver = new ResizeObserver(() => {
      if (!gridRef.current) return
      const children = Array.from(gridRef.current.querySelectorAll('.houdini-card'))
      children.forEach((child) => {
        child.style.minHeight = 'auto'
      })
      const max = children.reduce((height, child) => Math.max(height, child.offsetHeight), 0)
      children.forEach((child) => {
        child.style.minHeight = `${max}px`
      })
    })
    resizeObserver.observe(gridRef.current)
    return () => resizeObserver.disconnect()
  }, [supportsHoudini])

  return (
    <section className="houdini-panel" aria-label="Houdini layout showcase">
      <div className="houdini-heading">
        <div>
          <h2>Houdini Layout & Paint</h2>
          <p>
            Dynamic browser layout and card paint generated in the rendering engine for low-jank grids.
          </p>
        </div>
        <div className="houdini-tag">
          <span>{supportsHoudini ? 'Houdini active' : 'Fallback layout active'}</span>
        </div>
      </div>
      <div
        ref={gridRef}
        className={`houdini-grid ${supportsHoudini ? 'houdini-supported' : 'houdini-fallback'}`}
      >
        {cards.map((card, index) => (
          <article key={card.title} className="houdini-card" style={{ '--card-hue': card.accent }}>
            <div className="houdini-card-badge">{index + 1}</div>
            <h3>{card.title}</h3>
            <p>{card.subtitle}</p>
          </article>
        ))}
      </div>
    </section>
  )
}

export default HoudiniMasonry
