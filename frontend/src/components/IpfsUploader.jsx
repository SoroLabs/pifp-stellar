import { useState } from 'react'

const ORACLE_API = (import.meta.env.VITE_ORACLE_API_URL || 'http://localhost:9090/api').replace(/\/$/, '')

export function IpfsUploader() {
  const [file, setFile] = useState(null)
  const [isUploading, setIsUploading] = useState(false)
  const [cid, setCid] = useState('')
  const [error, setError] = useState('')

  const handleDragOver = (e) => e.preventDefault()
  
  const handleDrop = (e) => {
    e.preventDefault()
    const droppedFile = e.dataTransfer.files[0]
    if (droppedFile) setFile(droppedFile)
  }

  const handleUpload = async () => {
    if (!file) return
    setIsUploading(true)
    setError('')
    setCid('')

    const formData = new FormData()
    formData.append('file', file)

    try {
      const response = await fetch(`${ORACLE_API}/ipfs/upload`, {
        method: 'POST',
        body: formData,
      })

      if (!response.ok) {
        throw new Error(`Upload failed: ${response.statusText}`)
      }

      const data = await response.json()
      setCid(data.cid)
    } catch (err) {
      setError(err.message)
    } finally {
      setIsUploading(false)
    }
  }

  return (
    <div className="ipfs-uploader">
      <header className="page-header">
        <h2>Decentralized Metadata Storage</h2>
        <p>Attach rich media to your protocol actions using our native IPFS pinning service.</p>
      </header>

      <div 
        className={`drop-zone ${file ? 'has-file' : ''}`}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
      >
        {file ? (
          <div className="file-info">
            <span className="file-name">{file.name}</span>
            <span className="file-size">{(file.size / 1024).toFixed(2)} KB</span>
            <button className="remove-btn" onClick={() => setFile(null)}>✕</button>
          </div>
        ) : (
          <div className="prompt">
            <p>Drag and drop media here</p>
            <span>or click to select file</span>
            <input 
              type="file" 
              className="file-input" 
              onChange={(e) => setFile(e.target.files[0])}
            />
          </div>
        )}
      </div>

      <div className="actions">
        <button 
          onClick={handleUpload} 
          disabled={!file || isUploading}
          className="upload-btn"
        >
          {isUploading ? 'Streaming to IPFS...' : 'Pin to IPFS Cluster'}
        </button>
      </div>

      {cid && (
        <div className="upload-result">
          <p>✓ Successfully pinned!</p>
          <div className="cid-box">
            <code>ipfs://{cid}</code>
            <button onClick={() => navigator.clipboard.writeText(`ipfs://${cid}`)}>Copy</button>
          </div>
        </div>
      )}

      {error && <p className="error">{error}</p>}
    </div>
  )
}
