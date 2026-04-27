import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.jsx'
import { TelemetryProfiler } from './tracing/TelemetryProfiler'

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <TelemetryProfiler id="app-root">
      <App />
    </TelemetryProfiler>
  </StrictMode>,
)
