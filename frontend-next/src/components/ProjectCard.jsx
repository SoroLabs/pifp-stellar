'use client'

import Link from 'next/link'
import React from 'react'

export default function ProjectCard({
  id,
  title,
  description,
  category = 'Funding',
  current = 0,
  goal = 10000,
  donorCount = 12,
  token = 'XLM',
  gradient
}) {
  const progress = Math.min(Math.round((current / (goal || 1)) * 100), 100)

  const cardGradient = gradient || 'linear-gradient(135deg, #0284c7 0%, #0369a1 50%, #075985 100%)'

  return (
    <Link
      href={`/projects/${id}`}
      data-predictive="true"
      className="glass-panel project-card-interactive animate-fade-in"
      style={{
        display: 'flex',
        flexDirection: 'column',
        borderRadius: '16px',
        overflow: 'hidden',
        border: '1px solid var(--panel-border)',
        textDecoration: 'none',
        color: 'inherit',
        transition: 'transform 0.25s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.25s, box-shadow 0.25s',
        height: '100%',
        position: 'relative',
        background: 'var(--panel)'
      }}
    >
      {/* Decorative Visual Banner with SVG Geometry */}
      <div style={{
        height: '160px',
        background: cardGradient,
        position: 'relative',
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'flex-end',
        padding: '1.25rem'
      }}>
        {/* Abstract futuristic grid / circuitry overlay */}
        <svg
          style={{ position: 'absolute', inset: 0, opacity: 0.25, width: '100%', height: '100%' }}
          xmlns="http://www.w3.org/2000/svg"
        >
          <defs>
            <pattern id={`grid-${id}`} width="24" height="24" patternUnits="userSpaceOnUse">
              <path d="M 24 0 L 0 0 0 24" fill="none" stroke="white" strokeWidth="0.75" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill={`url(#grid-${id})`} />
          <circle cx="85%" cy="30%" r="50" fill="rgba(255,255,255,0.15)" filter="blur(20px)" />
        </svg>

        <div style={{ position: 'relative', zIndex: 2, display: 'flex', justifyContent: 'space-between', width: '100%', alignItems: 'center' }}>
          <span className={`pill ${category.toLowerCase()}`} style={{
            backdropFilter: 'blur(8px)',
            background: 'rgba(5, 11, 20, 0.75)',
            border: '1px solid rgba(255, 255, 255, 0.15)',
            boxShadow: '0 4px 12px rgba(0,0,0,0.3)'
          }}>
            <span className="ok-dot" style={{
              background: category === 'Completed' ? '#8b5cf6' :
                          category === 'Verified' ? '#10b981' :
                          category === 'Active' ? '#f59e0b' : '#00d2ff'
            }}></span>
            {category}
          </span>

          <span style={{
            fontSize: '0.75rem',
            fontWeight: 700,
            padding: '0.2rem 0.6rem',
            borderRadius: '6px',
            background: 'rgba(0,0,0,0.5)',
            color: 'rgba(255,255,255,0.9)',
            border: '1px solid rgba(255,255,255,0.1)'
          }}>
            {progress}% FUNDED
          </span>
        </div>
      </div>

      {/* Content Area */}
      <div style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', flex: 1 }}>
        <h3 style={{
          margin: '0 0 0.75rem 0',
          fontSize: '1.25rem',
          fontWeight: 700,
          color: '#fff',
          lineHeight: 1.3,
          letterSpacing: '-0.01em'
        }}>
          {title}
        </h3>

        <p className="muted" style={{
          margin: '0 0 1.5rem 0',
          fontSize: '0.88rem',
          lineHeight: 1.5,
          display: '-webkit-box',
          WebkitLineClamp: 2,
          WebkitBoxOrient: 'vertical',
          overflow: 'hidden',
          flex: 1
        }}>
          {description}
        </p>

        {/* Funding Progress Meter */}
        <div style={{ marginTop: 'auto' }}>
          <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'baseline',
            marginBottom: '0.5rem'
          }}>
            <div>
              <span style={{ fontSize: '1.2rem', fontWeight: 800, color: 'var(--accent)' }}>
                {current.toLocaleString()}
              </span>
              <span style={{ fontSize: '0.8rem', color: 'var(--muted)', marginLeft: '4px', fontWeight: 600 }}>
                {token}
              </span>
            </div>
            <div style={{ fontSize: '0.8rem', color: 'var(--muted)' }}>
              Goal: {goal.toLocaleString()} {token}
            </div>
          </div>

          {/* Progress bar track */}
          <div style={{
            width: '100%',
            height: '8px',
            background: 'rgba(255,255,255,0.08)',
            borderRadius: '4px',
            overflow: 'hidden',
            marginBottom: '1rem'
          }}>
            <div style={{
              width: `${progress}%`,
              height: '100%',
              background: 'linear-gradient(90deg, var(--primary), var(--accent))',
              borderRadius: '4px',
              transition: 'width 0.6s cubic-bezier(0.4, 0, 0.2, 1)'
            }} />
          </div>

          {/* Card footer info */}
          <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            fontSize: '0.8rem',
            color: 'var(--muted)',
            borderTop: '1px solid var(--glass-border)',
            paddingTop: '0.85rem'
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
                <circle cx="9" cy="7" r="4"/>
                <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
                <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
              </svg>
              <span>{donorCount} Backers</span>
            </div>

            <span style={{
              color: 'var(--accent)',
              fontWeight: 600,
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px'
            }}>
              View Protocol →
            </span>
          </div>
        </div>
      </div>
    </Link>
  )
}
