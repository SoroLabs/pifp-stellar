import { useEffect, useMemo, useState } from 'react'
import './App.css'
import { BridgeWatcher } from './components/BridgeWatcher'
import { IpfsUploader } from './components/IpfsUploader'
import { OTCInterface } from './components/OTCInterface'

const API_BASE = (import.meta.env.VITE_INDEXER_API_URL || 'http://localhost:8080').replace(/\/$/, '')

const SORT_FIELDS = [
  { value: 'created_ledger', label: 'Created Ledger' },
  { value: 'goal', label: 'Goal Amount' },
  { value: 'project_id', label: 'Project ID' },
  { value: 'creator', label: 'Creator' },
  { value: 'status', label: 'Status' },
]

function compareBigIntLike(a: any, b: any) {
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
  const [activeTab, setActiveTab] = useState('dashboard')
  const [projects, setProjects] = useState<any[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState('')

  const [status, setStatus] = useState('all')
  const [creator, setCreator] = useState('')
  const [category, setCategory] = useState('')
  const [sortField, setSortField] = useState('created_ledger')
  const [sortDirection, setSortDirection] = useState('desc')

  // OTC States
  const [roomId, setRoomId] = useState('')
  const [isInitiator, setIsInitiator] = useState(true)
  const [isJoined, setIsJoined] = useState(false)

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
      } catch (err: any) {
        if (err.name !== 'AbortError') {
          setError(err.message || 'Failed to fetch projects')
          setProjects([])
        }
      } finally {
        setIsLoading(false)
      }
    }

    if (activeTab === 'dashboard') {
      loadProjects()
    }
    return () => controller.abort()
  }, [status, creator, category, activeTab])

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
    <main className="app-container">
      <nav className="main-nav">
        <button 
          className={activeTab === 'dashboard' ? 'active' : ''} 
          onClick={() => setActiveTab('dashboard')}
        >
          Project Discovery
        </button>
        <button 
          className={activeTab === 'bridge' ? 'active' : ''} 
          onClick={() => setActiveTab('bridge')}
        >
          Bridge Watcher
        </button>
        <button 
          className={activeTab === 'ipfs' ? 'active' : ''} 
          onClick={() => setActiveTab('ipfs')}
        >
          IPFS Storage
        </button>
        <button 
          className={activeTab === 'otc' ? 'active' : ''} 
          onClick={() => setActiveTab('otc')}
        >
          OTC Trade (P2P)
        </button>
      </nav>

      {activeTab === 'dashboard' && (
        <section className="dashboard">
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
        </section>
      )}

      {activeTab === 'bridge' && <BridgeWatcher />}
      {activeTab === 'ipfs' && <IpfsUploader />}
      
      {activeTab === 'otc' && (
        <div className="otc-setup-container">
          {!isJoined ? (
            <div className="otc-join-card">
              <Zap size={48} color="#3b82f6" />
              <h2>Join OTC Negotiation</h2>
              <p>Direct peer-to-peer off-chain negotiation for Soroban assets. Secure, zero-knowledge signaling.</p>
              <div className="join-form">
                <input 
                  placeholder="Enter Room ID (e.g. trade-123)" 
                  value={roomId} 
                  onChange={e => setRoomId(e.target.value)} 
                />
                <div className="role-selector">
                    <button 
                        className={isInitiator ? 'active' : ''} 
                        onClick={() => setIsInitiator(true)}
                    >Initiator</button>
                    <button 
                        className={!isInitiator ? 'active' : ''} 
                        onClick={() => setIsInitiator(false)}
                    >Receiver</button>
                </div>
                <button 
                  className="join-btn"
                  onClick={() => roomId && setIsJoined(true)}
                >
                  Enter Negotiation Room
                </button>
              </div>
            </div>
          ) : (
            <OTCInterface roomId={roomId} isInitiator={isInitiator} />
          )}
        </div>
      )}

      <style>{`
        .otc-setup-container {
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 600px;
        }
        .otc-join-card {
            background: #1e293b;
            padding: 40px;
            border-radius: 16px;
            border: 1px solid #334155;
            text-align: center;
            max-width: 450px;
            width: 100%;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 20px;
            box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
        }
        .otc-join-card h2 { margin: 0; color: white; }
        .otc-join-card p { color: #94a3b8; font-size: 14px; }
        .join-form {
            width: 100%;
            display: flex;
            flex-direction: column;
            gap: 15px;
        }
        .join-form input {
            background: #0f172a;
            border: 1px solid #334155;
            color: white;
            padding: 12px;
            border-radius: 8px;
            text-align: center;
        }
        .role-selector {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 10px;
        }
        .role-selector button {
            background: #0f172a;
            border: 1px solid #334155;
            color: #94a3b8;
            padding: 10px;
            border-radius: 8px;
            cursor: pointer;
            transition: all 0.2s;
        }
        .role-selector button.active {
            background: #3b82f6;
            color: white;
            border-color: #3b82f6;
        }
        .join-btn {
            background: #3b82f6;
            color: white;
            border: none;
            padding: 14px;
            border-radius: 8px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s;
        }
        .join-btn:hover { background: #2563eb; transform: translateY(-1px); }
      `}</style>
    </main>
  )
}

export default App
