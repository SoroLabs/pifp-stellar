import Link from 'next/link'

export default function HomePage() {
  return (
    <main>
      <h1>Predictive Navigation Control Plane</h1>
      <p className="muted">
        Move your pointer toward a card to trigger millisecond-early prefetches. Hover and trajectory are both
        considered.
      </p>
      <div className="panel">
        <span className="pill">
          <span className="ok-dot" /> App Router + Service Worker Enabled
        </span>
      </div>
      <section className="panel">
        <h2>Complex Route Tree</h2>
        <div className="grid">
          <Link className="card" data-predictive="true" href="/projects/funding/alpha">
            Funding / Alpha
          </Link>
          <Link className="card" data-predictive="true" href="/projects/funding/bravo">
            Funding / Bravo
          </Link>
          <Link className="card" data-predictive="true" href="/projects/live/charlie">
            Live / Charlie
          </Link>
          <Link className="card" data-predictive="true" href="/projects/archive/delta">
            Archive / Delta
          </Link>
        </div>
      </section>
    </main>
  )
}
