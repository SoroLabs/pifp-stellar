import { useEffect, useEffectEvent, useMemo, useRef, useState } from 'react'
import {
  createCanvasSurface,
  destroyCanvasSurface,
  renderCanvasTree,
  resizeCanvasSurface,
} from '../charting/canvasRenderer'

const CHART_HEIGHT = 420
const HISTORY_LIMIT = 240
const UPDATE_BATCH_SIZE = 28
const UPDATE_INTERVAL_MS = 16

function formatCompact(value) {
  return new Intl.NumberFormat('en-US', {
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(value)
}

function formatPrice(value) {
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value)
}

function createInitialSeries() {
  const points = []
  const volumes = []
  let price = 102.2

  for (let index = 0; index < HISTORY_LIMIT; index += 1) {
    price += (Math.random() - 0.48) * 1.8
    const roundedPrice = Number(price.toFixed(2))
    points.push({ value: roundedPrice })
    volumes.push({
      value: 80 + Math.random() * 180,
      direction: index > 0 && roundedPrice >= points[index - 1].value ? 'up' : 'down',
    })
  }

  return {
    price: points.at(-1)?.value ?? 102.2,
    points,
    volumes,
  }
}

function clampSeries(points, nextPoint) {
  points.push(nextPoint)
  if (points.length > HISTORY_LIMIT) {
    points.shift()
  }
}

function TradingScene({ width, height, market, points, volumes, price, updatesPerSecond, batchSize }) {
  const minY = Math.min(...points.map((point) => point.value))
  const maxY = Math.max(...points.map((point) => point.value))
  const priceDelta = price - points[Math.max(0, points.length - 2)]?.value
  const trendColor = priceDelta >= 0 ? '#32d6ad' : '#ff7a7a'
  const crosshairX = width - 64

  return (
    <chart width={width} height={height} padding={30}>
      <grid verticalLines={8} horizontalLines={5} />
      <barSeries bars={volumes} barWidth={3.2} />
      <lineSeries
        points={points}
        minY={minY - 1.4}
        maxY={maxY + 1.4}
        stroke={trendColor}
        lineWidth={2.8}
        fill="rgba(50, 214, 173, 0.10)"
      />
      <crosshair x={crosshairX} label={`${market} LIVE`} />
      <label x={30} y={24} fill="#f8fafc" font='700 16px "Space Grotesk", sans-serif' text={`${market} orderflow`} />
      <label x={width - 30} y={24} fill="#94a3b8" align="right" text={`${updatesPerSecond}/s canvas updates`} />
      <label x={30} y={height - 18} fill="#e2e8f0" text={`Last ${formatPrice(price)}`} />
      <label
        x={width - 30}
        y={height - 18}
        fill={trendColor}
        align="right"
        text={`${batchSize} ticks / flush`}
      />
    </chart>
  )
}

export function HighFrequencyTradingChart() {
  const initialSeries = useMemo(() => createInitialSeries(), [])
  const canvasRef = useRef(null)
  const hostRef = useRef(null)
  const surfaceRef = useRef(null)
  const seriesRef = useRef(initialSeries)
  const frameRef = useRef({
    pendingTicks: 0,
    totalTicks: 0,
    lastFlushAt: 0,
    updatesPerSecond: 0,
  })
  const [stats, setStats] = useState({
    price: initialSeries.price,
    totalTicks: 0,
    updatesPerSecond: 0,
    points: HISTORY_LIMIT,
  })

  const renderScene = useEffectEvent(() => {
    if (!surfaceRef.current || !hostRef.current) {
      return
    }

    const width = Math.max(620, Math.floor(hostRef.current.clientWidth))
    const { points, volumes, price } = seriesRef.current
    const { updatesPerSecond } = frameRef.current

    resizeCanvasSurface(surfaceRef.current, width, CHART_HEIGHT)
    renderCanvasTree(
      surfaceRef.current,
      <TradingScene
        width={width}
        height={CHART_HEIGHT}
        market="XLM / USDC"
        points={points}
        volumes={volumes}
        price={price}
        updatesPerSecond={updatesPerSecond}
        batchSize={UPDATE_BATCH_SIZE}
      />,
    )
  })

  const flushTickBatch = useEffectEvent(() => {
    const series = seriesRef.current
    const frame = frameRef.current
    let price = series.price

    for (let index = 0; index < UPDATE_BATCH_SIZE; index += 1) {
      const drift = (Math.random() - 0.49) * 1.12
      price = Number(Math.max(96, Math.min(112, price + drift)).toFixed(2))
      const previousPoint = series.points.at(-1)
      clampSeries(series.points, { value: price })
      clampSeries(series.volumes, {
        value: 60 + Math.random() * 210,
        direction: !previousPoint || price >= previousPoint.value ? 'up' : 'down',
      })
    }

    series.price = price
    frame.pendingTicks += UPDATE_BATCH_SIZE
    frame.totalTicks += UPDATE_BATCH_SIZE

    const now = performance.now()
    const elapsed = now - frame.lastFlushAt
    if (elapsed >= 1000) {
      frame.updatesPerSecond = Math.round((frame.pendingTicks / elapsed) * 1000)
      frame.pendingTicks = 0
      frame.lastFlushAt = now
    }

    renderScene()
  })

  useEffect(() => {
    if (!canvasRef.current) {
      return undefined
    }

    surfaceRef.current = createCanvasSurface(canvasRef.current, {
      width: Math.max(620, hostRef.current?.clientWidth ?? 620),
      height: CHART_HEIGHT,
      padding: 30,
    })
    frameRef.current.lastFlushAt = performance.now()

    renderScene()

    const resizeObserver = new ResizeObserver(() => {
      renderScene()
    })

    if (hostRef.current) {
      resizeObserver.observe(hostRef.current)
    }

    const tickInterval = window.setInterval(() => {
      flushTickBatch()
    }, UPDATE_INTERVAL_MS)

    const statsInterval = window.setInterval(() => {
      setStats({
        price: seriesRef.current.price,
        totalTicks: frameRef.current.totalTicks,
        updatesPerSecond: frameRef.current.updatesPerSecond,
        points: seriesRef.current.points.length,
      })
    }, 250)

    return () => {
      resizeObserver.disconnect()
      window.clearInterval(tickInterval)
      window.clearInterval(statsInterval)
      if (surfaceRef.current) {
        destroyCanvasSurface(surfaceRef.current)
        surfaceRef.current = null
      }
    }
  }, [])

  const statCards = useMemo(
    () => [
      { label: 'Render path', value: 'React Reconciler -> Canvas' },
      { label: 'Ticks processed', value: formatCompact(stats.totalTicks) },
      { label: 'Live throughput', value: `${formatCompact(stats.updatesPerSecond)} / sec` },
      { label: 'Last price', value: `$${formatPrice(stats.price)}` },
    ],
    [stats],
  )

  return (
    <section className="trading-shell">
      <header className="trading-hero">
        <p className="eyebrow">Custom Renderer</p>
        <h1>High-frequency market chart without React DOM churn</h1>
        <p className="subhead">
          This chart is rendered by a dedicated React reconciler that translates chart primitives
          directly into Canvas commands. Tick bursts stay isolated inside the canvas surface, so
          the rest of the app avoids high-frequency DOM diffing.
        </p>
      </header>

      <section className="trading-meta">
        {statCards.map((card) => (
          <article className="metric-card" key={card.label}>
            <span>{card.label}</span>
            <strong>{card.value}</strong>
          </article>
        ))}
      </section>

      <section className="chart-stage" ref={hostRef}>
        <div className="chart-stage__header">
          <div>
            <p className="eyebrow">Isolated Updates</p>
            <h2>Canvas-backed trading tape</h2>
          </div>
          <div className="stage-pill">batched @ {UPDATE_INTERVAL_MS}ms</div>
        </div>
        <canvas ref={canvasRef} aria-label="High-frequency trading chart rendered on canvas" />
      </section>

      <section className="implementation-notes">
        <article>
          <h3>Reconciler host</h3>
          <p>
            The renderer defines host primitives like <code>lineSeries</code>, <code>barSeries</code>,
            <code> grid</code>, and <code>label</code>, then commits them to a lightweight scene graph.
          </p>
        </article>
        <article>
          <h3>Canvas abstraction</h3>
          <p>
            Commit completion schedules a single <code>requestAnimationFrame</code> draw, which paints
            the scene graph with the 2D Canvas API instead of producing DOM nodes.
          </p>
        </article>
        <article>
          <h3>UI integration</h3>
          <p>
            The chart lives inside a normal React component, but market ticks stream through the custom
            renderer directly so sibling tabs and dashboard UI stay unaffected.
          </p>
        </article>
      </section>
    </section>
  )
}
