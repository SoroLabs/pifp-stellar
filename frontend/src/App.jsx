import { useEffect, useMemo, useState } from 'react'
import './App.css'
import { BridgeWatcher } from './components/BridgeWatcher'
import { HighFrequencyTradingChart } from './components/HighFrequencyTradingChart'
import { IpfsUploader } from './components/IpfsUploader'
import VerifiedBadge from './components/VerifiedBadge'
import { verifyMerkleProof } from './utils/merkle'
import DebtVisualizer from './components/DebtVisualizer'
import { ApolloProvider, useQuery } from '@apollo/client'
import client from './graphql/client'
import { GET_PROJECTS } from './graphql/queries'
import RealtimeActivity from './components/RealtimeActivity'
import BondingCurveSimulator from './components/BondingCurveSimulator'

const API_BASE = (import.meta.env.VITE_INDEXER_API_URL || 'http://localhost:8080').replace(/\/$/, '')
const ORACLE_API = 'http://localhost:9090/api/offchain'

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
  return (
    <ApolloProvider client={client}>
      <AppContent />
    </ApolloProvider>
  )
}

function AppContent() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [projects, setProjects] = useState([])
  const [error, setError] = useState('')

  const [currentView, setCurrentView] = useState('dashboard')

  const [status, setStatus] = useState('all')
  const [creator, setCreator] = useState('')
  const [category, setCategory] = useState('')
  const [sortField, setSortField] = useState('created_ledger')
  const [sortDirection, setSortDirection] = useState('desc')
  const [verificationResults, setVerificationResults] = useState({}) // { projectId: { isVerified, result, stateRoot, ledgerSeq } }
  const [isVerifying, setIsVerifying] = useState({})

  const { data, loading: isLoading, error: queryError } = useQuery(GET_PROJECTS, {
    variables: {
      status: status === 'all' ? undefined : status,
      creator: creator.trim() || undefined,
      limit: 200,
    },
    skip: activeTab !== 'dashboard'
  })

  useEffect(() => {
    if (data?.projects) {
      setProjects(data.projects)
    }
  }, [data])

  useEffect(() => {
    if (queryError) {
      setError(queryError.message)
    }
  }, [queryError])

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

  const handleVerify = async (projectId) => {
    setIsVerifying(prev => ({ ...prev, [projectId]: true }))
    try {
      const response = await fetch(`${ORACLE_API}/compute?project_id=${projectId}`)
      if (!response.ok) throw new Error('Failed to fetch off-chain result')
      
      const data = await response.json()
      
      // Verify all proofs
      let allValid = true
      for (const proof of data.proofs) {
        const isValid = await verifyMerkleProof(data.state_root, proof.leaf, proof.index, proof.siblings)
        if (!isValid) {
          allValid = false
          break
        }
      }

      setVerificationResults(prev => ({
        ...prev,
        [projectId]: {
          isVerified: allValid,
          result: data.result,
          stateRoot: data.state_root,
          ledgerSeq: data.ledger_seq
        }
      }))
    } catch (err) {
      console.error(err)
      setVerificationResults(prev => ({
        ...prev,
        [projectId]: { isVerified: false, result: 'Verification Error' }
      }))
    } finally {
      setIsVerifying(prev => ({ ...prev, [projectId]: false }))
    }
  }

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
          className={activeTab === 'trading' ? 'active' : ''}
          onClick={() => setActiveTab('trading')}
        >
          Trading Canvas
        </button>
        <button 
          className={activeTab === 'ipfs' ? 'active' : ''} 
          onClick={() => setActiveTab('ipfs')}
        >
          IPFS Storage
        </button>
        <button 
          className={activeTab === 'debt' ? 'active' : ''} 
          onClick={() => setActiveTab('debt')}
        >
          Debt Optimizer
        </button>
        <button 
          className={activeTab === 'tokenomics' ? 'active' : ''} 
          onClick={() => setActiveTab('tokenomics')}
        >
          Tokenomics
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
                      <th>Off-chain Computation</th>
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
                        <td className="offchain-cell">
                          {verificationResults[project.project_id] ? (
                            <div className="v-result">
                              <span className="v-text">{verificationResults[project.project_id].result}</span>
                              <VerifiedBadge 
                                isVerified={verificationResults[project.project_id].isVerified}
                                stateRoot={verificationResults[project.project_id].stateRoot}
                                ledgerSeq={verificationResults[project.project_id].ledgerSeq}
                              />
                            </div>
                          ) : (
                            <button 
                              className="verify-btn"
                              onClick={() => handleVerify(project.project_id)}
                              disabled={isVerifying[project.project_id]}
                            >
                              {isVerifying[project.project_id] ? 'Verifying...' : 'Run & Verify'}
                            </button>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {!isLoading && !error && sortedProjects.length === 0 && (
              <p className="state">No projects matched current filters.</p>
            )}

            <RealtimeActivity />
          </section>
        </section>
      ) : currentView === 'orderbook' ? (
        <section style={{ padding: '20px', display: 'flex', justifyContent: 'center', marginTop: '20px' }}>
          <Orderbook />
        </section>
      ) : currentView === 'explorer' ? (
        <section style={{ padding: '20px', display: 'flex', justifyContent: 'center', marginTop: '20px' }}>
          <VirtualGrid />
        </section>
      ) : (
        <>
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

      {activeTab === 'bridge' && <BridgeWatcher />}
      {activeTab === 'trading' && <HighFrequencyTradingChart />}
      {activeTab === 'ipfs' && <IpfsUploader />}
      {activeTab === 'debt' && <DebtVisualizer />}
      {activeTab === 'tokenomics' && <BondingCurveSimulator />}
    </main>
  )
}

export default App
