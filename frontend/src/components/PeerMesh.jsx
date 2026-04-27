import { useEffect, useMemo, useRef, useState } from 'react'

const SIGNALING_URL = import.meta.env.VITE_SIGNALING_URL || 'wss://signal.pifp.dev'
const MAX_PEERS = 5
const STUN_SERVERS = [{ urls: 'stun:stun.l.google.com:19302' }]

function safeJsonParse(raw) {
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}

function PeerMesh() {
  const [status, setStatus] = useState('Initializing peer mesh...')
  const [connectedPeers, setConnectedPeers] = useState([])
  const [messages, setMessages] = useState([])
  const [localLog, setLocalLog] = useState([])

  const wsRef = useRef(null)
  const connections = useRef(new Map())
  const channels = useRef(new Map())
  const pendingCandidates = useRef(new Map())
  const peerId = useMemo(() => `peer-${Math.random().toString(16).slice(2)}`, [])

  const log = (entry) => {
    setLocalLog((prev) => [...prev.slice(-20), `${new Date().toISOString().slice(11, 19)} ${entry}`])
  }

  const sendSignal = (payload) => {
    const ws = wsRef.current
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ peerId, ...payload }))
    }
  }

  const updatePeerList = () => {
    setConnectedPeers(Array.from(connections.current.keys()).sort())
  }

  const createConnection = async (targetId, isInitiator) => {
    if (connections.current.has(targetId)) {
      return connections.current.get(targetId)
    }

    const pc = new RTCPeerConnection({ iceServers: STUN_SERVERS })
    connections.current.set(targetId, pc)
    updatePeerList()

    pc.onicecandidate = (event) => {
      if (event.candidate) {
        sendSignal({ type: 'ice-candidate', targetId, candidate: event.candidate })
      }
    }

    pc.onconnectionstatechange = () => {
      if (pc.connectionState === 'disconnected' || pc.connectionState === 'failed' || pc.connectionState === 'closed') {
        pc.close()
        connections.current.delete(targetId)
        channels.current.delete(targetId)
        updatePeerList()
        log(`Peer ${targetId} disconnected (${pc.connectionState})`)
      }
    }

    pc.ondatachannel = (event) => {
      const channel = event.channel
      channel.onopen = () => {
        log(`Received data channel from ${targetId}`)
        channels.current.set(targetId, channel)
        updatePeerList()
        channel.send(JSON.stringify({ type: 'hello', text: 'Hello from ' + peerId }))
      }
      channel.onmessage = (event) => {
        setMessages((prev) => [...prev, { peer: targetId, payload: event.data }])
      }
    }

    if (isInitiator) {
      const channel = pc.createDataChannel('pifp-mempool', { ordered: true })
      channel.onopen = () => {
        log(`Opened data channel to ${targetId}`)
        channels.current.set(targetId, channel)
        updatePeerList()
        channel.send(JSON.stringify({ type: 'hello', text: 'Active peer connection established' }))
      }
      channel.onmessage = (event) => {
        setMessages((prev) => [...prev, { peer: targetId, payload: event.data }])
      }
      channel.onclose = () => {
        channels.current.delete(targetId)
        updatePeerList()
      }
    }

    const queued = pendingCandidates.current.get(targetId) || []
    pendingCandidates.current.delete(targetId)
    for (const candidate of queued) {
      pc.addIceCandidate(candidate).catch(() => {
        log(`Failed to add queued ICE candidate for ${targetId}`)
      })
    }

    return pc
  }

  const handleOffer = async ({ peerId: remoteId, sdp }) => {
    if (remoteId === peerId || connections.current.size >= MAX_PEERS) {
      return
    }
    log(`Received offer from ${remoteId}`)
    const pc = await createConnection(remoteId, false)
    await pc.setRemoteDescription(new RTCSessionDescription(sdp))
    const answer = await pc.createAnswer()
    await pc.setLocalDescription(answer)
    sendSignal({ type: 'answer', targetId: remoteId, sdp: pc.localDescription })
  }

  const handleAnswer = async ({ peerId: remoteId, sdp }) => {
    log(`Received answer from ${remoteId}`)
    const pc = connections.current.get(remoteId)
    if (!pc) return
    await pc.setRemoteDescription(new RTCSessionDescription(sdp))
  }

  const handleCandidate = async ({ peerId: remoteId, candidate }) => {
    if (!connections.current.has(remoteId)) {
      pendingCandidates.current.set(remoteId, [...(pendingCandidates.current.get(remoteId) || []), candidate])
      return
    }
    const pc = connections.current.get(remoteId)
    await pc.addIceCandidate(new RTCIceCandidate(candidate)).catch(() => {
      log(`ICE candidate rejected from ${remoteId}`)
    })
  }

  const handlePeerHello = async ({ peerId: remoteId }) => {
    if (remoteId === peerId || connections.current.has(remoteId) || connections.current.size >= MAX_PEERS) {
      return
    }
    const pc = await createConnection(remoteId, true)
    const offer = await pc.createOffer()
    await pc.setLocalDescription(offer)
    sendSignal({ type: 'offer', targetId: remoteId, sdp: pc.localDescription })
    log(`Sent offer to ${remoteId}`)
  }

  useEffect(() => {
    const ws = new WebSocket(SIGNALING_URL)
    wsRef.current = ws

    ws.onopen = () => {
      setStatus(`Connected to signaling server at ${SIGNALING_URL}`)
      log('Signaling channel open')
      sendSignal({ type: 'hello' })
    }

    ws.onmessage = (message) => {
      const payload = safeJsonParse(message.data)
      if (!payload || payload.peerId === peerId) {
        return
      }
      switch (payload.type) {
        case 'hello':
          handlePeerHello(payload)
          break
        case 'offer':
          handleOffer(payload)
          break
        case 'answer':
          handleAnswer(payload)
          break
        case 'ice-candidate':
          handleCandidate(payload)
          break
        default:
          log(`Unknown signaling payload: ${payload.type}`)
      }
    }

    ws.onclose = () => {
      setStatus('Signaling connection closed')
      log('Signaling channel closed')
    }

    ws.onerror = () => {
      setStatus('Signaling connection error')
      log('Signaling error, check network or signaling endpoint.')
    }

    return () => {
      ws.close()
      connections.current.forEach((pc) => pc.close())
    }
  }, [])

  useEffect(() => {
    const interval = setInterval(() => {
      if (channels.current.size > 0) {
        for (const [remoteId, channel] of channels.current.entries()) {
          if (channel.readyState === 'open') {
            channel.send(JSON.stringify({ type: 'ping', timestamp: Date.now(), from: peerId }))
          }
        }
      }
    }, 15000)
    return () => clearInterval(interval)
  }, [])

  return (
    <section className="mesh-panel" aria-label="Peer discovery mesh">
      <div className="mesh-header">
        <div>
          <h2>Peer Discovery Mesh</h2>
          <p>{status}</p>
        </div>
        <div className="mesh-stats">
          <strong>{connectedPeers.length}</strong>
          <span>active peer{connectedPeers.length === 1 ? '' : 's'}</span>
        </div>
      </div>
      <div className="mesh-block">
        <div className="mesh-card">
          <h3>Local Peer ID</h3>
          <code>{peerId}</code>
          <p>Max peers allowed: {MAX_PEERS}</p>
        </div>
        <div className="mesh-card">
          <h3>Connected Peers</h3>
          <ul>
            {connectedPeers.length > 0 ? (
              connectedPeers.map((id) => <li key={id}>{id}</li>)
            ) : (
              <li>No connected peers yet</li>
            )}
          </ul>
        </div>
      </div>
      <div className="mesh-log">
        <h3>Mesh Events</h3>
        <pre>{[...localLog].reverse().join('\n')}</pre>
      </div>
      <div className="mesh-messages">
        <h3>Recent Data Messages</h3>
        <ul>
          {messages.slice(-8).map((item, index) => (
            <li key={`${item.peer}-${index}`}>[{item.peer}] {item.payload}</li>
          ))}
        </ul>
      </div>
    </section>
  )
}

export default PeerMesh
