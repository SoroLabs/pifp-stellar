'use client'

import React, { createContext, useContext, useState, useEffect } from 'react'

const INITIAL_PROJECTS = [
  {
    id: 'ocean-cleanup',
    title: 'Global Ocean Cleanup Initiative',
    description: 'Deploying autonomous solar-powered marine drones to intercept and remove microplastics from high-density ocean gyres.',
    longDescription: 'The Great Pacific Garbage patch spans millions of square kilometers. This initiative utilizes Soroban-controlled decentralized automated recovery units (DARUs) powered by clean solar energy. Every kilogram of waste collected is verifiable via telemetry sensors attested by independent environmental oracles on the Stellar ledger.',
    category: 'Funding',
    token: 'XLM',
    current: 48500,
    goal: 100000,
    donorCount: 142,
    creator: 'GDROP4W5E6Q4XJ2PVR7N5O9K1L8M3A2B',
    deadline: '28 days left',
    gradient: 'linear-gradient(135deg, #0284c7 0%, #0369a1 50%, #075985 100%)',
    contractAddress: 'CA7QW...479201PIFP',
    oracleThreshold: '2 of 3 Oracles',
    milestones: [
      {
        id: 1,
        title: 'Phase 1: Autonomous Marine Fleet Deployment',
        amount: 25000,
        bps: 2500,
        status: 'completed',
        proofHash: '0x8f2a9e...4b12',
        description: 'Deploy 5 autonomous drone vessels with satellite telemetry to target coordinates.'
      },
      {
        id: 2,
        title: 'Phase 2: Sensor Telemetry & Plastic Extraction',
        amount: 45000,
        bps: 4500,
        status: 'in-progress',
        proofHash: '0x3c71db...99ef',
        description: 'Collect initial 50 metric tons of marine plastic verified by Marine Research Oracle.'
      },
      {
        id: 3,
        title: 'Phase 3: Circular Economy Recycling Pipeline',
        amount: 30000,
        bps: 3000,
        status: 'locked',
        proofHash: '0x000000...0000',
        description: 'Transport collected polymers to certified repurposing facilities in coastal hubs.'
      }
    ],
    recentDonations: [
      { donor: 'GBX9...4821', amount: 1200, time: '12 mins ago', txHash: '0x7e8b...99a1' },
      { donor: 'GCK2...9103', amount: 3500, time: '1 hour ago', txHash: '0x14d0...bb54' },
      { donor: 'GART...5529', amount: 500, time: '3 hours ago', txHash: '0x99fe...02cd' }
    ]
  },
  {
    id: 'solar-schools',
    title: 'Solar Panels for Off-Grid Rural Schools',
    description: 'Providing sustainable solar microgrids, battery storage, and Starlink connectivity to 50 off-grid schools in Sub-Saharan Africa.',
    longDescription: 'Over 60% of rural schools in the targeted districts lack reliable electricity, preventing students from using digital learning tools. This project installs robust 10kW solar arrays and lithium battery storage packs, empowering over 15,000 students with evening lighting and internet access.',
    category: 'Active',
    token: 'XLM',
    current: 12000,
    goal: 12000,
    donorCount: 88,
    creator: 'GAF334MNBVX9801KJHGFDSAQWERT56',
    deadline: 'Goal Reached - Execution Active',
    gradient: 'linear-gradient(135deg, #d97706 0%, #b45309 50%, #78350f 100%)',
    contractAddress: 'CC934...118832PIFP',
    oracleThreshold: '3 of 5 Oracles',
    milestones: [
      {
        id: 1,
        title: 'Hardware Procurement & Freight Logistics',
        amount: 5000,
        bps: 4166,
        status: 'completed',
        proofHash: '0x55aa11...88cc',
        description: 'Solar panels and inverters cleared through customs and delivered to local depot.'
      },
      {
        id: 2,
        title: 'Installation at 25 Primary Schools',
        amount: 4000,
        bps: 3333,
        status: 'completed',
        proofHash: '0x66bb22...99dd',
        description: 'First 25 solar systems operational and verified by local municipal inspector.'
      },
      {
        id: 3,
        title: 'Full 50-School Grid Commissioning',
        amount: 3000,
        bps: 2501,
        status: 'in-progress',
        proofHash: '0x77cc33...00ee',
        description: 'Final commissioning and Starlink dish integration across all 50 educational sites.'
      }
    ],
    recentDonations: [
      { donor: 'GDAP...3321', amount: 2000, time: '2 days ago', txHash: '0xaa12...33fe' },
      { donor: 'GMLP...0099', amount: 500, time: '3 days ago', txHash: '0xbb34...44aa' }
    ]
  },
  {
    id: 'quantum-research',
    title: 'Open Source Quantum Error Mitigation for Soroban',
    description: 'Developing cryptographic libraries that harden smart contracts against future quantum computing attacks using lattice cryptography.',
    longDescription: 'Quantum computers pose future risks to elliptic curve cryptography. This open-source Soroban library introduces post-quantum signatures (Dilithium & Kyber) optimized for the resource-constrained WebAssembly runtime of Stellar Soroban.',
    category: 'Funding',
    token: 'XLM',
    current: 18500,
    goal: 50000,
    donorCount: 64,
    creator: 'GQTM998811KLOPIUYTREWQASDFGHJKL',
    deadline: '45 days left',
    gradient: 'linear-gradient(135deg, #7c3aed 0%, #6d28d9 50%, #4c1d95 100%)',
    contractAddress: 'CQTM7...992200PIFP',
    oracleThreshold: '2 of 2 Oracles',
    milestones: [
      {
        id: 1,
        title: 'WASM Runtime Profiling & Optimization',
        amount: 15000,
        bps: 3000,
        status: 'completed',
        proofHash: '0x12ff34...56aa',
        description: 'Benchmark lattice verification operations under 100M CPU instructions in Soroban.'
      },
      {
        id: 2,
        title: 'Formally Verified Rust Crate',
        amount: 20000,
        bps: 4000,
        status: 'in-progress',
        proofHash: '0x99aa88...77bb',
        description: 'Security audit and formal mathematical verification of polynomial multiplications.'
      },
      {
        id: 3,
        title: 'Public Testnet Developer SDK',
        amount: 15000,
        bps: 3000,
        status: 'locked',
        proofHash: '0x000000...0000',
        description: 'Publish documentation and ready-to-use Soroban smart contract templates.'
      }
    ],
    recentDonations: [
      { donor: 'GZAB...8812', amount: 5000, time: '4 hours ago', txHash: '0x88ee...11ff' },
      { donor: 'GPOI...6622', amount: 1500, time: '1 day ago', txHash: '0x3344...5566' }
    ]
  },
  {
    id: 'amazon-reforestation',
    title: 'Indigenous Rainforest Guardian Reforestation',
    description: 'Partnering directly with indigenous communities in the Madre de Dios basin to plant and monitor 500,000 native canopy trees.',
    longDescription: 'Combining ancient indigenous forestry wisdom with modern satellite and drone multispectral imaging. Seedlings are tracked individually, and funding is unlocked automatically as canopy growth thresholds are confirmed via satellite oracle feeds.',
    category: 'Verified',
    token: 'XLM',
    current: 250000,
    goal: 250000,
    donorCount: 630,
    creator: 'GIND990011223344556677889900AABB',
    deadline: 'Milestones Verified - Funds in Escrow',
    gradient: 'linear-gradient(135deg, #059669 0%, #047857 50%, #064e3b 100%)',
    contractAddress: 'CFST1...883311PIFP',
    oracleThreshold: '3 of 4 Oracles',
    milestones: [
      {
        id: 1,
        title: 'Nursery Setup & 200k Native Seedlings',
        amount: 100000,
        bps: 4000,
        status: 'completed',
        proofHash: '0xaabbcc...1122',
        description: 'Community greenhouses constructed and nursery inventory verified.'
      },
      {
        id: 2,
        title: 'Direct Planting in Buffer Zone',
        amount: 100000,
        bps: 4000,
        status: 'completed',
        proofHash: '0x334455...6677',
        description: '350 hectares planted with diverse native timber and fruit species.'
      },
      {
        id: 3,
        title: 'Sentinel-2 Satellite Biomass Verification',
        amount: 50000,
        bps: 2000,
        status: 'completed',
        proofHash: '0x889900...aabb',
        description: 'Satellite vegetation index confirms 92% canopy survival rate.'
      }
    ],
    recentDonations: [
      { donor: 'GMAX...7777', amount: 10000, time: '5 hours ago', txHash: '0xcafe...babe' },
      { donor: 'GPLN...2211', amount: 5000, time: '8 hours ago', txHash: '0x1234...5678' }
    ]
  },
  {
    id: 'clean-water-stellar',
    title: 'Solar UV Water Purification Kiosks',
    description: 'Installing automated solar-powered UV water filtration stations that supply affordable potable water to 12 rural health centers.',
    longDescription: 'Water-borne illnesses account for a major portion of pediatric clinic visits. These self-contained UV filtration kiosks operate continuously, logging real-time flow rate and water purity metrics directly to the Stellar ledger via lightweight IoT bridges.',
    category: 'Funding',
    token: 'XLM',
    current: 18400,
    goal: 35000,
    donorCount: 79,
    creator: 'GH2O8811447722558833669911AABBCC',
    deadline: '19 days left',
    gradient: 'linear-gradient(135deg, #0891b2 0%, #0e7490 50%, #155e75 100%)',
    contractAddress: 'CH2O5...229944PIFP',
    oracleThreshold: '2 of 3 Oracles',
    milestones: [
      {
        id: 1,
        title: 'Civil Works & Wellhead Inspection',
        amount: 10000,
        bps: 2857,
        status: 'completed',
        proofHash: '0xaa1122...3344',
        description: 'Structural inspection and contamination testing at all 12 candidate wellheads.'
      },
      {
        id: 2,
        title: 'Solar UV Kiosk Assembly & Logistics',
        amount: 15000,
        bps: 4286,
        status: 'in-progress',
        proofHash: '0x556677...8899',
        description: 'Deploy stainless-steel filtration skids and solar arrays to primary clinics.'
      },
      {
        id: 3,
        title: 'IoT Purity Telemetry Integration',
        amount: 10000,
        bps: 2857,
        status: 'locked',
        proofHash: '0x000000...0000',
        description: 'Connect LoRaWAN telemetry nodes for automatic on-chain water quality proofs.'
      }
    ],
    recentDonations: [
      { donor: 'GWTR...4499', amount: 2500, time: '20 mins ago', txHash: '0x55ee...9900' },
      { donor: 'GCLU...1122', amount: 800, time: '2 hours ago', txHash: '0x77aa...1122' }
    ]
  },
  {
    id: 'open-soroban-tools',
    title: 'Soroban Studio: Interactive Smart Contract IDE',
    description: 'Building a zero-install, in-browser developer studio and testing sandbox for Soroban smart contracts on the Stellar network.',
    longDescription: 'Lowering the barrier to entry for Stellar developers. Soroban Studio provides one-click contract deployment to Testnet, visual invocation panels, parameter serialization inspection, and automated unit test generation.',
    category: 'Completed',
    token: 'XLM',
    current: 80000,
    goal: 80000,
    donorCount: 310,
    creator: 'GDEV11223344556677889900AABBCCDD',
    deadline: 'Project Complete & Shipped',
    gradient: 'linear-gradient(135deg, #4f46e5 0%, #4338ca 50%, #312e81 100%)',
    contractAddress: 'CDEV9...771144PIFP',
    oracleThreshold: '2 of 3 Oracles',
    milestones: [
      {
        id: 1,
        title: 'WASM Compiler in Browser (WebAssembly)',
        amount: 25000,
        bps: 3125,
        status: 'completed',
        proofHash: '0x991122...3344',
        description: 'Execute Soroban SDK compilation directly in modern browsers via Rust WASM.'
      },
      {
        id: 2,
        title: 'RPC Simulator & Testnet Integration',
        amount: 30000,
        bps: 3750,
        status: 'completed',
        proofHash: '0x445566...7788',
        description: 'Full Horizon and Soroban RPC bridge supporting simulated contract states.'
      },
      {
        id: 3,
        title: 'Public Open Beta Launch',
        amount: 25000,
        bps: 3125,
        status: 'completed',
        proofHash: '0x889900...aabb',
        description: 'Production release supporting 5,000+ monthly active developers.'
      }
    ],
    recentDonations: [
      { donor: 'GRPC...9900', amount: 5000, time: '3 weeks ago', txHash: '0xbb11...99aa' }
    ]
  }
]

const AppContext = createContext(null)

export function AppProvider({ children }) {
  // Wallet state
  const [wallet, setWallet] = useState(() => {
    if (typeof window !== 'undefined') {
      try {
        const savedWallet = localStorage.getItem('pifp_wallet')
        if (savedWallet) return JSON.parse(savedWallet)
      } catch {}
    }
    return {
      isConnected: false,
      address: '',
      shortAddress: '',
      balance: 2450.5,
      network: 'Stellar Testnet',
      walletType: ''
    }
  })

  // Projects state
  const [projects, setProjects] = useState(() => {
    if (typeof window !== 'undefined') {
      try {
        const savedProjects = localStorage.getItem('pifp_projects')
        if (savedProjects) return JSON.parse(savedProjects)
      } catch {}
    }
    return INITIAL_PROJECTS
  })

  // Transactions state
  const [transactions, setTransactions] = useState([])

  // Modal controls
  const [isWalletModalOpen, setWalletModalOpen] = useState(false)
  const [isCreateModalOpen, setCreateModalOpen] = useState(false)

  // Toasts
  const [toasts, setToasts] = useState([])

  // Centralized Stellar ledger heartbeat
  const [ledger, setLedger] = useState({
    latestLedger: 100000,
    latestLedgerCloseMs: 0
  })
  const [pulse, setPulse] = useState(false)

  useEffect(() => {
    let timerId
    let cancelled = false

    const pollLedger = async () => {
      // Don't poll when page is hidden in background
      if (typeof document !== 'undefined' && document.hidden) {
        timerId = setTimeout(pollLedger, 5000)
        return
      }

      try {
        const response = await fetch('/api/ledger/latest', { cache: 'no-store' })
        if (response.ok && !cancelled) {
          const payload = await response.json()
          setLedger(payload)
          setPulse(true)
          setTimeout(() => { if (!cancelled) setPulse(false) }, 1000)

          // Inform Service Worker for cache freshness invalidation
          if (typeof navigator !== 'undefined' && navigator.serviceWorker?.controller) {
            navigator.serviceWorker.controller.postMessage({
              type: 'LEDGER_UPDATE',
              latestLedgerCloseMs: payload.latestLedgerCloseMs
            })
          }
        }
      } catch {
        // Ignore transient network failures
      } finally {
        if (!cancelled) {
          timerId = setTimeout(pollLedger, 5000)
        }
      }
    }

    pollLedger()

    return () => {
      cancelled = true
      clearTimeout(timerId)
    }
  }, [])

  // Sync projects
  const saveProjects = (newProjects) => {
    setProjects(newProjects)
    try {
      localStorage.setItem('pifp_projects', JSON.stringify(newProjects))
    } catch {}
  }

  const addToast = (title, message, type = 'success') => {
    const id = Date.now() + Math.random().toString(36).substring(2, 5)
    setToasts(prev => [...prev, { id, title, message, type }])
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id))
    }, 4500)
  }

  const removeToast = (id) => {
    setToasts(prev => prev.filter(t => t.id !== id))
  }

  const connectWallet = (type = 'Freighter') => {
    const mockAddress = 'GBX2R7U9W4K3L6M1Q8P5N0A4D7C2E9B'
    const newWallet = {
      isConnected: true,
      address: mockAddress,
      shortAddress: `${mockAddress.slice(0, 4)}...${mockAddress.slice(-4)}`,
      balance: 3820.75,
      network: 'Stellar Testnet (Soroban)',
      walletType: type
    }
    setWallet(newWallet)
    try {
      localStorage.setItem('pifp_wallet', JSON.stringify(newWallet))
    } catch {}
    setWalletModalOpen(false)
    addToast('Wallet Connected', `Connected via ${type} on Stellar Testnet.`, 'success')
  }

  const disconnectWallet = () => {
    const emptyWallet = {
      isConnected: false,
      address: '',
      shortAddress: '',
      balance: 0,
      network: 'Stellar Testnet',
      walletType: ''
    }
    setWallet(emptyWallet)
    try {
      localStorage.removeItem('pifp_wallet')
    } catch {}
    addToast('Disconnected', 'Your wallet has been disconnected.', 'info')
  }

  const createProject = (projectData) => {
    const slug = projectData.title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || `project-${Date.now()}`

    const gradients = [
      'linear-gradient(135deg, #0284c7 0%, #0369a1 50%, #075985 100%)',
      'linear-gradient(135deg, #10b981 0%, #059669 50%, #047857 100%)',
      'linear-gradient(135deg, #8b5cf6 0%, #7c3aed 50%, #6d28d9 100%)',
      'linear-gradient(135deg, #f59e0b 0%, #d97706 50%, #b45309 100%)',
      'linear-gradient(135deg, #ec4899 0%, #db2777 50%, #be185d 100%)'
    ]
    const randomGradient = gradients[Math.floor(Math.random() * gradients.length)]

    const newProject = {
      id: slug,
      title: projectData.title,
      description: projectData.description,
      longDescription: projectData.longDescription || projectData.description,
      category: 'Funding',
      token: projectData.token || 'XLM',
      current: 0,
      goal: Number(projectData.goal) || 10000,
      donorCount: 0,
      creator: wallet.address || 'GCREATOR99887766554433221100AABB',
      deadline: `${projectData.deadlineDays || 30} days left`,
      gradient: randomGradient,
      contractAddress: `CPIFP...${Math.random().toString(36).substring(2, 8).toUpperCase()}`,
      oracleThreshold: `${projectData.oracleThreshold || '2 of 3'} Oracles`,
      milestones: projectData.milestones && projectData.milestones.length > 0
        ? projectData.milestones
        : [
            {
              id: 1,
              title: 'Phase 1: Initial Implementation & Setup',
              amount: Math.round(Number(projectData.goal) * 0.4),
              bps: 4000,
              status: 'in-progress',
              proofHash: '0x000000...0000',
              description: 'Initial project execution and milestones verification.'
            },
            {
              id: 2,
              title: 'Phase 2: Completion & Final Deliverables',
              amount: Math.round(Number(projectData.goal) * 0.6),
              bps: 6000,
              status: 'locked',
              proofHash: '0x000000...0000',
              description: 'Final deliverables verified by authorized Stellar oracles.'
            }
          ],
      recentDonations: []
    }

    const updated = [newProject, ...projects]
    saveProjects(updated)
    setCreateModalOpen(false)
    addToast('Project Registered!', `"${newProject.title}" deployed to Soroban protocol.`, 'success')
    return slug
  }

  const donateToProject = (projectId, amount) => {
    const numAmount = Number(amount)
    if (isNaN(numAmount) || numAmount <= 0) {
      addToast('Invalid Amount', 'Please enter a valid donation amount in XLM.', 'error')
      return false
    }

    if (wallet.isConnected && wallet.balance < numAmount) {
      addToast('Insufficient Balance', `You need ${numAmount} XLM, but your wallet only has ${wallet.balance} XLM.`, 'error')
      return false
    }

    // Deduct from wallet if connected
    if (wallet.isConnected) {
      setWallet(prev => {
        const next = { ...prev, balance: Math.max(0, +(prev.balance - numAmount).toFixed(2)) }
        try { localStorage.setItem('pifp_wallet', JSON.stringify(next)) } catch {}
        return next
      })
    }

    const txHash = `0x${Array.from({ length: 16 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`
    const donorDisplay = wallet.isConnected ? wallet.shortAddress : 'GUEST...DONOR'

    const updated = projects.map(p => {
      if (p.id === projectId) {
        const nextCurrent = p.current + numAmount
        const nextStatus = nextCurrent >= p.goal && p.category === 'Funding' ? 'Active' : p.category
        return {
          ...p,
          current: nextCurrent,
          category: nextStatus,
          donorCount: p.donorCount + 1,
          recentDonations: [
            { donor: donorDisplay, amount: numAmount, time: 'Just now', txHash },
            ...(p.recentDonations || [])
          ]
        }
      }
      return p
    })

    saveProjects(updated)

    setTransactions(prev => [
      {
        id: txHash,
        type: 'DONATION',
        projectId,
        amount: numAmount,
        time: new Date().toLocaleTimeString(),
        status: 'Confirmed (Ledger Enrolled)'
      },
      ...prev
    ])

    addToast('Donation Confirmed!', `Contributed ${numAmount.toLocaleString()} XLM via Soroban Escrow.`, 'success')
    return true
  }

  return (
    <AppContext.Provider
      value={{
        wallet,
        projects,
        transactions,
        isWalletModalOpen,
        isCreateModalOpen,
        toasts,
        ledger,
        pulse,
        setWalletModalOpen,
        setCreateModalOpen,
        connectWallet,
        disconnectWallet,
        createProject,
        donateToProject,
        addToast,
        removeToast
      }}
    >
      {children}
    </AppContext.Provider>
  )
}

export function useApp() {
  const context = useContext(AppContext)
  if (!context) {
    throw new Error('useApp must be used within an AppProvider')
  }
  return context
}
