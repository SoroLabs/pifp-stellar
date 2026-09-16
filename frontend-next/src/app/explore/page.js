'use client'

import { useState } from 'react'
import ProjectCard from '@/components/ProjectCard'
import { useApp } from '@/context/AppContext'

export default function ExplorePage() {
  const { projects, setCreateModalOpen } = useApp()

  const [filter, setFilter] = useState('All')
  const [searchQuery, setSearchQuery] = useState('')
  const [sortBy, setSortBy] = useState('most-funded')

  // Filter projects
  const filtered = projects.filter(project => {
    const matchesCategory = filter === 'All' || project.category.toLowerCase() === filter.toLowerCase()
    const matchesSearch =
      project.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      project.description.toLowerCase().includes(searchQuery.toLowerCase())
    return matchesCategory && matchesSearch
  })

  // Sort projects
  const sorted = [...filtered].sort((a, b) => {
    if (sortBy === 'most-funded') {
      return (b.current / b.goal) - (a.current / a.goal)
    }
    if (sortBy === 'highest-goal') {
      return b.goal - a.goal
    }
    if (sortBy === 'backers') {
      return b.donorCount - a.donorCount
    }
    return 0
  })

  return (
    <div className="container animate-fade-in" style={{ padding: '2rem 1.5rem 6rem' }}>
      {/* Page Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'flex-start',
        marginBottom: '3rem',
        flexWrap: 'wrap',
        gap: '1.5rem',
        borderBottom: '1px solid var(--glass-border)',
        paddingBottom: '2.5rem'
      }}>
        <div>
          <div className="pill funding" style={{ marginBottom: '1rem' }}>
            <span className="ok-dot" /> Verified Impact Registry
          </div>
          <h1 style={{ fontSize: 'clamp(2.2rem, 4vw, 3.2rem)', margin: '0 0 0.75rem 0', color: '#fff' }}>
            Explore Impact Initiatives
          </h1>
          <p className="muted" style={{ fontSize: '1.1rem', margin: 0, maxWidth: '640px', lineHeight: 1.5 }}>
            Discover and back verified global public good protocols running on Stellar. All funds are secured by smart contract escrow milestones.
          </p>
        </div>

        <button
          onClick={() => setCreateModalOpen(true)}
          className="btn btn-accent"
          style={{ padding: '0.8rem 1.8rem', gap: '8px' }}
        >
          <span>+</span>
          <span>Propose Initiative</span>
        </button>
      </div>

      {/* Filter and Search Controls Bar */}
      <div style={{
        display: 'flex',
        flexWrap: 'wrap',
        justifyContent: 'space-between',
        alignItems: 'center',
        gap: '1.25rem',
        marginBottom: '2.5rem'
      }}>
        {/* Category Tabs */}
        <div style={{ display: 'flex', gap: '0.6rem', flexWrap: 'wrap' }}>
          {['All', 'Funding', 'Active', 'Verified', 'Completed'].map(cat => {
            const isSelected = filter.toLowerCase() === cat.toLowerCase()
            return (
              <button
                key={cat}
                onClick={() => setFilter(cat)}
                className={`btn ${isSelected ? 'btn-primary' : 'btn-outline'}`}
                style={{
                  borderRadius: '999px',
                  padding: '0.45rem 1.2rem',
                  fontSize: '0.88rem',
                  fontWeight: 600,
                  border: isSelected ? 'none' : '1px solid rgba(255, 255, 255, 0.1)'
                }}
              >
                {cat}
              </button>
            )
          })}
        </div>

        {/* Search & Sort Controls */}
        <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'center' }}>
          {/* Search Input */}
          <div style={{ position: 'relative', minWidth: '260px' }}>
            <span style={{
              position: 'absolute',
              left: '12px',
              top: '50%',
              transform: 'translateY(-50%)',
              color: 'var(--muted)',
              fontSize: '0.9rem'
            }}>
              🔍
            </span>
            <input
              type="text"
              placeholder="Search by keywords..."
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              style={{
                width: '100%',
                padding: '0.6rem 1rem 0.6rem 2.2rem',
                borderRadius: '999px',
                border: '1px solid var(--glass-border)',
                background: 'rgba(0, 0, 0, 0.3)',
                color: '#fff',
                fontSize: '0.9rem',
                outline: 'none'
              }}
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                style={{
                  position: 'absolute',
                  right: '12px',
                  top: '50%',
                  transform: 'translateY(-50%)',
                  color: 'var(--muted)',
                  fontSize: '0.9rem'
                }}
              >
                ✕
              </button>
            )}
          </div>

          {/* Sort Selector */}
          <select
            value={sortBy}
            onChange={e => setSortBy(e.target.value)}
            style={{
              padding: '0.6rem 1rem',
              borderRadius: '999px',
              border: '1px solid var(--glass-border)',
              background: '#09101d',
              color: 'var(--text)',
              fontSize: '0.88rem',
              fontWeight: 600,
              outline: 'none',
              cursor: 'pointer'
            }}
          >
            <option value="most-funded">Sort: % Funded</option>
            <option value="highest-goal">Sort: Highest Goal</option>
            <option value="backers">Sort: Most Backers</option>
          </select>
        </div>
      </div>

      {/* Results Count */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '1.5rem',
        fontSize: '0.88rem',
        color: 'var(--muted)'
      }}>
        <span>Showing {sorted.length} {sorted.length === 1 ? 'initiative' : 'initiatives'}</span>
        {filter !== 'All' && <span>Filtered by: <strong>{filter}</strong></span>}
      </div>

      {/* Projects Grid */}
      <div className="grid grid-cols-3">
        {sorted.map(project => (
          <ProjectCard key={project.id} {...project} />
        ))}
      </div>

      {/* Empty State */}
      {sorted.length === 0 && (
        <div className="glass-panel" style={{
          padding: '4rem 2rem',
          textAlign: 'center',
          marginTop: '2rem',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '1.25rem'
        }}>
          <div style={{ fontSize: '3rem' }}>🔎</div>
          <h3 style={{ margin: 0, fontSize: '1.4rem', color: '#fff' }}>No initiatives found</h3>
          <p className="muted" style={{ margin: 0, maxWidth: '400px' }}>
            No projects matched your current search filters. Try adjusting your query or clear the active category filter.
          </p>
          <button
            onClick={() => { setFilter('All'); setSearchQuery(''); }}
            className="btn btn-outline"
            style={{ marginTop: '0.5rem' }}
          >
            Reset All Filters
          </button>
        </div>
      )}
    </div>
  )
}
