import LedgerRoutePanel from '@/components/LedgerRoutePanel'

export default function ProjectDetailsPage({ params }) {
  const { status, slug } = params

  return (
    <main>
      <h1>
        Project {slug} ({status})
      </h1>
      <p className="muted">Route payloads for this page are targeted by predictive prefetch + SW caching.</p>
      <section className="panel">
        <h2>Ledger Freshness Gate</h2>
        <LedgerRoutePanel />
      </section>
    </main>
  )
}
