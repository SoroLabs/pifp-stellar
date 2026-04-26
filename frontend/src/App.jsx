import { useEffect, useMemo, useState } from 'react'
import './App.css'
import { DidWallet } from './components/DidWallet'
import { ZkProver } from './components/ZkProver'

const API_BASE = (import.meta.env.VITE_INDEXER_API_URL || 'http://localhost:8080').replace(/\/$/, '')

const SORT_FIELDS = [
  { value: 'created_ledger', label: 'Created Ledger' },
  { value: 'goal', label: 'Goal Amount' },
  { value: 'project_id', label: 'Project ID' },
  { value: 'creator', label: 'Creator' },
  { value: 'status', label: 'Status' },
]

function compareBigIntLike(a, b) {
  try {
    const aBig = BigInt(a ?? 0)
    const bBig = BigInt(b ?? 0)
    if (aBig < bBig) return -1
    if (aBig > bBig) return 1
    return 0
  } catch {
    return String(a ?? '').localeCompare(String(b ?? ''))
  }
}

function App() {
  const [projects, setProjects] = useState([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState('')

  const [status, setStatus] = useState('all')
  const [creator, setCreator] = useState('')
  const [category, setCategory] = useState('')
  const [sortField, setSortField] = useState('created_ledger')
  const [sortDirection, setSortDirection] = useState('desc')

  useEffect(() => {
    const controller = new AbortController()

    async function loadProjects() {
      setIsLoading(true)
      setError('')
      try {
        const params = new URLSearchParams({ limit: '200', offset: '0' })
        if (status !== 'all') params.set('status', status)
        if (creator.trim()) params.set('creator', creator.trim())
        if (category.trim()) params.set('category', category.trim())

        const response = await fetch(`${API_BASE}/projects?${params.toString()}`, {
          signal: controller.signal,
        })

        if (!response.ok) {
          throw new Error(`Indexer returned ${response.status}`)
        }

        const payload = await response.json()
        setProjects(Array.isArray(payload.projects) ? payload.projects : [])
      } catch (err) {
        if (err.name !== 'AbortError') {
          setError(err.message || 'Failed to fetch projects')
          setProjects([])
        }
      } finally {
        setIsLoading(false)
      }
    }

    loadProjects()
    return () => controller.abort()
  }, [status, creator, category])

  const sortedProjects = useMemo(() => {
    const items = [...projects]
    items.sort((a, b) => {
      let result = 0
      if (sortField === 'created_ledger' || sortField === 'goal') {
        result = compareBigIntLike(a[sortField], b[sortField])
      } else {
        result = String(a[sortField] ?? '').localeCompare(String(b[sortField] ?? ''))
      }
      return sortDirection === 'asc' ? result : -result
    })
    return items
  }, [projects, sortField, sortDirection])

  return (
    <main className="dashboard">
      <header className="hero">
        <p className="eyebrow">PIFP Stellar Indexer</p>
        <h1>Project Discovery Dashboard</h1>
        <p className="subhead">
          Live view of indexed projects with quick filters and sorting controls.
        </p>
      </header>

      <section className="filters" aria-label="Project filters">
        <label>
          <span>Status</span>
          <select value={status} onChange={(e) => setStatus(e.target.value)}>
            <option value="all">All</option>
            <option value="Funding">Funding</option>
            <option value="Active">Active</option>
            <option value="Completed">Completed</option>
            <option value="Expired">Expired</option>
          </select>
        </label>

        <label>
          <span>Creator</span>
          <input
            value={creator}
            onChange={(e) => setCreator(e.target.value)}
            placeholder="G... address"
          />
        </label>

        <label>
          <span>Category</span>
          <input
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            placeholder="edu,health"
          />
        </label>

        <label>
          <span>Sort By</span>
          <select value={sortField} onChange={(e) => setSortField(e.target.value)}>
            {SORT_FIELDS.map((field) => (
              <option key={field.value} value={field.value}>
                {field.label}
              </option>
            ))}
          </select>
        </label>

        <label>
          <span>Direction</span>
          <select
            value={sortDirection}
            onChange={(e) => setSortDirection(e.target.value)}
          >
            <option value="desc">Descending</option>
            <option value="asc">Ascending</option>
          </select>
        </label>
      </section>

      <section className="results" aria-live="polite">
        <div className="results-bar">
          <strong>{sortedProjects.length}</strong>
          <span>projects</span>
        </div>

        {isLoading && <p className="state">Loading projects...</p>}
        {error && !isLoading && <p className="state error">{error}</p>}

        {!isLoading && !error && (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Project ID</th>
                  <th>Status</th>
                  <th>Creator</th>
                  <th>Goal</th>
                  <th>Primary Token</th>
                  <th>Created Ledger</th>
                </tr>
              </thead>
              <tbody>
                {sortedProjects.map((project) => (
                  <tr key={project.project_id}>
                    <td>{project.project_id}</td>
                    <td>
                      <span className="pill">{project.status}</span>
                    </td>
                    <td className="truncate" title={project.creator}>
                      {project.creator}
                    </td>
                    <td>{project.goal}</td>
                    <td>{project.primary_token}</td>
                    <td>{project.created_ledger}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {!isLoading && !error && sortedProjects.length === 0 && (
          <p className="state">No projects matched current filters.</p>
        )}
      </section>

      <section className="identity-section">
        <DidWallet />
      </section>

      <section className="identity-section">
        <ZkProver />
      </section>
    </main>
  )
}

export default App
