import { useState, useRef, useEffect } from 'react'
import { createHelia } from 'helia'
import { http } from '@helia/http'

export function IpfsVideoStreamer({ cid }) {
  const videoRef = useRef()
  const [helia, setHelia] = useState()

  useEffect(() => {
    const initHelia = async () => {
      const h = await createHelia({ transports: [http()] })
      setHelia(h)
    }
    initHelia()
  }, [])

  const loadVideo = async () => {
    if (!helia || !cid) return
    // Simplified: fetch full file
    const response = await fetch(`https://ipfs.io/ipfs/${cid}`)
    const buffer = await response.arrayBuffer()
    const blob = new Blob([buffer], { type: 'video/mp4' })
    videoRef.current.src = URL.createObjectURL(blob)
  }

  useEffect(() => {
    if (helia && cid) {
      loadVideo()
    }
  }, [helia, cid])

  return (
    <div className="ipfs-video">
      <h2>IPFS Video Streaming</h2>
      <video ref={videoRef} controls />
    </div>
  )
}
