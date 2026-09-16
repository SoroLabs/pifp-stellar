'use client'

import React, { useState } from 'react'
import Modal from './Modal'
import { useApp } from '@/context/AppContext'
import { useRouter } from 'next/navigation'

export default function CreateProjectModal() {
  const { isCreateModalOpen, setCreateModalOpen, createProject, wallet, setWalletModalOpen } = useApp()
  const router = useRouter()

  const [formData, setFormData] = useState({
    title: '',
    category: 'Clean Energy & Climate',
    goal: '',
    deadlineDays: '30',
    description: '',
    longDescription: '',
    oracleThreshold: '2 of 3'
  })

  const [isDeploying, setIsDeploying] = useState(false)
  const [deployStep, setDeployStep] = useState(0)

  const steps = [
    'Validating project milestones and goal bounds...',
    'Generating Soroban cryptographic proof hashes...',
    'Submitting register_project invocation to Stellar Testnet...',
    'Confirming ledger enrolment...'
  ]

  const handleChange = (e) => {
    const { name, value } = e.target
    setFormData(prev => ({ ...prev, [name]: value }))
  }

  const handleSubmit = (e) => {
    e.preventDefault()
    if (!formData.title || !formData.goal || !formData.description) return

    if (!wallet.isConnected) {
      setWalletModalOpen(true)
      return
    }

    setIsDeploying(true)
    setDeployStep(0)

    const interval = setInterval(() => {
      setDeployStep(prev => {
        if (prev >= steps.length - 1) {
          clearInterval(interval)
          setTimeout(() => {
            const slug = createProject(formData)
            setIsDeploying(false)
            setFormData({
              title: '',
              category: 'Clean Energy & Climate',
              goal: '',
              deadlineDays: '30',
              description: '',
              longDescription: '',
              oracleThreshold: '2 of 3'
            })
            router.push(`/projects/${slug}`)
          }, 800)
          return prev
        }
        return prev + 1
      })
    }, 700)
  }

  return (
    <Modal
      isOpen={isCreateModalOpen}
      onClose={() => !isDeploying && setCreateModalOpen(false)}
      title="Deploy Public Impact Project"
      maxWidth="620px"
    >
      {isDeploying ? (
        <div style={{ padding: '2.5rem 1rem', textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1.5rem' }}>
          <div style={{
            width: '56px',
            height: '56px',
            borderRadius: '50%',
            border: '4px solid rgba(0, 210, 255, 0.15)',
            borderTopColor: 'var(--accent)',
            animation: 'spin 0.8s cubic-bezier(0.4, 0, 0.2, 1) infinite'
          }}></div>
          <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
          
          <div>
            <h3 style={{ margin: '0 0 0.5rem 0', fontSize: '1.25rem' }}>Deploying to Soroban</h3>
            <p className="muted" style={{ margin: 0, fontSize: '0.95rem' }}>
              {steps[deployStep]}
            </p>
          </div>

          <div style={{ width: '100%', maxWidth: '360px', height: '6px', background: 'rgba(255,255,255,0.1)', borderRadius: '3px', overflow: 'hidden' }}>
            <div style={{
              width: `${((deployStep + 1) / steps.length) * 100}%`,
              height: '100%',
              background: 'linear-gradient(90deg, var(--primary), var(--accent))',
              transition: 'width 0.5s ease'
            }}></div>
          </div>
        </div>
      ) : (
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
          <p className="muted" style={{ margin: 0, fontSize: '0.9rem', lineHeight: 1.5 }}>
            Create an impact initiative secured by Soroban milestone escrows. Funds are locked transparently on the Stellar ledger until verified.
          </p>

          <div>
            <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.4rem', color: 'var(--text)' }}>
              Project Title *
            </label>
            <input
              type="text"
              name="title"
              required
              placeholder="e.g. Decentralized Solar Microgrid for Coastal Fisheries"
              value={formData.title}
              onChange={handleChange}
              style={{
                width: '100%',
                padding: '0.75rem 1rem',
                borderRadius: '8px',
                border: '1px solid var(--glass-border)',
                background: 'rgba(0, 0, 0, 0.3)',
                color: '#fff',
                fontSize: '0.95rem',
                outline: 'none'
              }}
            />
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
            <div>
              <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.4rem', color: 'var(--text)' }}>
                Funding Goal (XLM) *
              </label>
              <input
                type="number"
                name="goal"
                required
                min="100"
                placeholder="25000"
                value={formData.goal}
                onChange={handleChange}
                style={{
                  width: '100%',
                  padding: '0.75rem 1rem',
                  borderRadius: '8px',
                  border: '1px solid var(--glass-border)',
                  background: 'rgba(0, 0, 0, 0.3)',
                  color: '#fff',
                  fontSize: '0.95rem',
                  outline: 'none'
                }}
              />
            </div>

            <div>
              <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.4rem', color: 'var(--text)' }}>
                Duration (Days)
              </label>
              <input
                type="number"
                name="deadlineDays"
                min="7"
                max="365"
                value={formData.deadlineDays}
                onChange={handleChange}
                style={{
                  width: '100%',
                  padding: '0.75rem 1rem',
                  borderRadius: '8px',
                  border: '1px solid var(--glass-border)',
                  background: 'rgba(0, 0, 0, 0.3)',
                  color: '#fff',
                  fontSize: '0.95rem',
                  outline: 'none'
                }}
              />
            </div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
            <div>
              <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.4rem', color: 'var(--text)' }}>
                Category
              </label>
              <select
                name="category"
                value={formData.category}
                onChange={handleChange}
                style={{
                  width: '100%',
                  padding: '0.75rem 1rem',
                  borderRadius: '8px',
                  border: '1px solid var(--glass-border)',
                  background: '#09101d',
                  color: '#fff',
                  fontSize: '0.95rem',
                  outline: 'none'
                }}
              >
                <option value="Clean Energy & Climate">Clean Energy & Climate</option>
                <option value="Education & Connectivity">Education & Connectivity</option>
                <option value="Open Source & Tooling">Open Source & Tooling</option>
                <option value="Healthcare & Water">Healthcare & Water</option>
              </select>
            </div>

            <div>
              <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.4rem', color: 'var(--text)' }}>
                Oracle Consensus
              </label>
              <select
                name="oracleThreshold"
                value={formData.oracleThreshold}
                onChange={handleChange}
                style={{
                  width: '100%',
                  padding: '0.75rem 1rem',
                  borderRadius: '8px',
                  border: '1px solid var(--glass-border)',
                  background: '#09101d',
                  color: '#fff',
                  fontSize: '0.95rem',
                  outline: 'none'
                }}
              >
                <option value="2 of 3">2 of 3 Oracles (Recommended)</option>
                <option value="3 of 5">3 of 5 Oracles (High Assurance)</option>
                <option value="1 of 1">1 of 1 Oracle (Fast Track)</option>
              </select>
            </div>
          </div>

          <div>
            <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.4rem', color: 'var(--text)' }}>
              Short Pitch *
            </label>
            <input
              type="text"
              name="description"
              required
              placeholder="Brief one-sentence description of the initiative"
              value={formData.description}
              onChange={handleChange}
              style={{
                width: '100%',
                padding: '0.75rem 1rem',
                borderRadius: '8px',
                border: '1px solid var(--glass-border)',
                background: 'rgba(0, 0, 0, 0.3)',
                color: '#fff',
                fontSize: '0.95rem',
                outline: 'none'
              }}
            />
          </div>

          <div>
            <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.4rem', color: 'var(--text)' }}>
              Detailed Implementation Plan & Impact Objectives
            </label>
            <textarea
              name="longDescription"
              rows="3"
              placeholder="Explain technical execution, verifiable milestones, and expected real-world impact..."
              value={formData.longDescription}
              onChange={handleChange}
              style={{
                width: '100%',
                padding: '0.75rem 1rem',
                borderRadius: '8px',
                border: '1px solid var(--glass-border)',
                background: 'rgba(0, 0, 0, 0.3)',
                color: '#fff',
                fontSize: '0.95rem',
                outline: 'none',
                resize: 'vertical'
              }}
            />
          </div>

          <div style={{ display: 'flex', gap: '1rem', marginTop: '0.5rem' }}>
            <button
              type="button"
              className="btn btn-outline"
              style={{ flex: 1, padding: '0.85rem' }}
              onClick={() => setCreateModalOpen(false)}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-accent"
              style={{ flex: 2, padding: '0.85rem', fontWeight: 700 }}
            >
              {wallet.isConnected ? 'Deploy to Soroban Protocol' : 'Connect Wallet to Deploy'}
            </button>
          </div>
        </form>
      )}
    </Modal>
  )
}
