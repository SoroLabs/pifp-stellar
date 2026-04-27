import ReactReconciler from 'react-reconciler'
import { ConcurrentRoot, DefaultEventPriority } from 'react-reconciler/constants'

const TEXT_INSTANCE = 'TEXT_INSTANCE'

let currentUpdatePriority = DefaultEventPriority

function shallowDiffProps(oldProps, newProps) {
  const oldKeys = Object.keys(oldProps)
  const newKeys = Object.keys(newProps)

  if (oldKeys.length !== newKeys.length) {
    return true
  }

  for (const key of newKeys) {
    if (key === 'children') {
      continue
    }

    if (oldProps[key] !== newProps[key]) {
      return true
    }
  }

  return false
}

function normalizeProps(props) {
  const nextProps = {}

  for (const key of Object.keys(props)) {
    if (key === 'children') {
      continue
    }
    nextProps[key] = props[key]
  }

  return nextProps
}

function appendChildNode(parent, child) {
  parent.children.push(child)
  child.parent = parent
}

function insertChildNode(parent, child, beforeChild) {
  const index = parent.children.indexOf(beforeChild)
  if (index === -1) {
    appendChildNode(parent, child)
    return
  }

  parent.children.splice(index, 0, child)
  child.parent = parent
}

function removeChildNode(parent, child) {
  const index = parent.children.indexOf(child)
  if (index !== -1) {
    parent.children.splice(index, 1)
  }
  child.parent = null
}

function scheduleDraw(container) {
  if (!container.context || container.frameId !== null) {
    return
  }

  container.frameId = window.requestAnimationFrame(() => {
    container.frameId = null
    drawContainer(container)
  })
}

function getScaleX(width, padding, totalPoints) {
  const innerWidth = Math.max(1, width - padding * 2)
  return totalPoints <= 1 ? innerWidth : innerWidth / (totalPoints - 1)
}

function drawGrid(ctx, node, chartFrame) {
  const { padding, width, height } = chartFrame
  const verticalLines = node.props.verticalLines ?? 8
  const horizontalLines = node.props.horizontalLines ?? 5
  const stroke = node.props.stroke ?? 'rgba(148, 163, 184, 0.18)'

  ctx.save()
  ctx.strokeStyle = stroke
  ctx.lineWidth = node.props.lineWidth ?? 1

  for (let x = 0; x <= verticalLines; x += 1) {
    const xPos = padding + ((width - padding * 2) / verticalLines) * x
    ctx.beginPath()
    ctx.moveTo(xPos, padding)
    ctx.lineTo(xPos, height - padding)
    ctx.stroke()
  }

  for (let y = 0; y <= horizontalLines; y += 1) {
    const yPos = padding + ((height - padding * 2) / horizontalLines) * y
    ctx.beginPath()
    ctx.moveTo(padding, yPos)
    ctx.lineTo(width - padding, yPos)
    ctx.stroke()
  }

  ctx.restore()
}

function drawLineSeries(ctx, node, chartFrame) {
  const points = Array.isArray(node.props.points) ? node.props.points : []
  if (points.length === 0) {
    return
  }

  const { width, height, padding } = chartFrame
  const innerHeight = height - padding * 2
  const minY = node.props.minY ?? Math.min(...points.map((point) => point.value))
  const maxY = node.props.maxY ?? Math.max(...points.map((point) => point.value))
  const range = Math.max(1, maxY - minY)
  const scaleX = getScaleX(width, padding, points.length)

  ctx.save()
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'
  ctx.lineWidth = node.props.lineWidth ?? 2
  ctx.strokeStyle = node.props.stroke ?? '#30c6a8'

  if (node.props.fill) {
    ctx.beginPath()
    points.forEach((point, index) => {
      const x = padding + scaleX * index
      const y = height - padding - ((point.value - minY) / range) * innerHeight
      if (index === 0) {
        ctx.moveTo(x, y)
      } else {
        ctx.lineTo(x, y)
      }
    })
    ctx.lineTo(padding + scaleX * (points.length - 1), height - padding)
    ctx.lineTo(padding, height - padding)
    ctx.closePath()
    ctx.fillStyle = node.props.fill
    ctx.fill()
  }

  ctx.beginPath()
  points.forEach((point, index) => {
    const x = padding + scaleX * index
    const y = height - padding - ((point.value - minY) / range) * innerHeight
    if (index === 0) {
      ctx.moveTo(x, y)
    } else {
      ctx.lineTo(x, y)
    }
  })
  ctx.stroke()
  ctx.restore()
}

function drawBarSeries(ctx, node, chartFrame) {
  const bars = Array.isArray(node.props.bars) ? node.props.bars : []
  if (bars.length === 0) {
    return
  }

  const { width, height, padding } = chartFrame
  const innerHeight = height - padding * 2
  const barWidth = node.props.barWidth ?? 4
  const maxValue = Math.max(1, ...bars.map((bar) => bar.value))
  const scaleX = getScaleX(width, padding, bars.length)

  ctx.save()
  bars.forEach((bar, index) => {
    const x = padding + scaleX * index - barWidth / 2
    const barHeight = (bar.value / maxValue) * innerHeight * 0.24
    const y = height - padding - barHeight
    ctx.fillStyle =
      bar.color ??
      (bar.direction === 'up' ? 'rgba(72, 187, 120, 0.45)' : 'rgba(248, 113, 113, 0.45)')
    ctx.fillRect(x, y, barWidth, barHeight)
  })
  ctx.restore()
}

function drawLabel(ctx, node) {
  const text =
    node.props.text ??
    node.children
      .filter((child) => child.type === TEXT_INSTANCE)
      .map((child) => child.text)
      .join('')

  if (!text) {
    return
  }

  ctx.save()
  ctx.font = node.props.font ?? '12px "Space Grotesk", sans-serif'
  ctx.fillStyle = node.props.fill ?? '#d8e5ec'
  ctx.textAlign = node.props.align ?? 'left'
  ctx.textBaseline = node.props.baseline ?? 'middle'
  ctx.fillText(text, node.props.x ?? 0, node.props.y ?? 0)
  ctx.restore()
}

function drawCrosshair(ctx, node, chartFrame) {
  const { x = 0, label } = node.props
  const { padding, height } = chartFrame

  ctx.save()
  ctx.strokeStyle = node.props.stroke ?? 'rgba(255, 255, 255, 0.18)'
  ctx.setLineDash([6, 6])
  ctx.beginPath()
  ctx.moveTo(x, padding)
  ctx.lineTo(x, height - padding)
  ctx.stroke()

  if (label) {
    ctx.setLineDash([])
    ctx.fillStyle = 'rgba(15, 23, 42, 0.88)'
    ctx.fillRect(x - 42, padding - 8, 84, 22)
    ctx.fillStyle = '#f8fafc'
    ctx.font = '11px "Space Grotesk", sans-serif'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(label, x, padding + 3)
  }

  ctx.restore()
}

function drawNode(ctx, node, chartFrame) {
  if (node.hidden) {
    return
  }

  switch (node.type) {
    case 'chart':
    case 'group':
      node.children.forEach((child) => drawNode(ctx, child, chartFrame))
      break
    case 'grid':
      drawGrid(ctx, node, chartFrame)
      break
    case 'lineSeries':
      drawLineSeries(ctx, node, chartFrame)
      break
    case 'barSeries':
      drawBarSeries(ctx, node, chartFrame)
      break
    case 'label':
      drawLabel(ctx, node, chartFrame)
      break
    case 'crosshair':
      drawCrosshair(ctx, node, chartFrame)
      break
    default:
      node.children.forEach((child) => drawNode(ctx, child, chartFrame))
  }
}

function drawContainer(container) {
  const ctx = container.context
  if (!ctx || !container.canvas) {
    return
  }

  const width = container.width
  const height = container.height
  const padding = container.padding

  ctx.clearRect(0, 0, width, height)

  const background = container.background ?? '#08131a'
  const gradient = ctx.createLinearGradient(0, 0, 0, height)
  gradient.addColorStop(0, background)
  gradient.addColorStop(1, '#091d28')
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, width, height)

  ctx.fillStyle = 'rgba(255, 255, 255, 0.03)'
  ctx.fillRect(padding, padding, width - padding * 2, height - padding * 2)

  const chartFrame = { width, height, padding }
  container.children.forEach((child) => drawNode(ctx, child, chartFrame))
}

function resizeSurface(container, canvas, width, height) {
  const pixelRatio = window.devicePixelRatio || 1
  canvas.width = Math.round(width * pixelRatio)
  canvas.height = Math.round(height * pixelRatio)
  canvas.style.width = `${width}px`
  canvas.style.height = `${height}px`

  const ctx = canvas.getContext('2d')
  ctx.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0)

  container.canvas = canvas
  container.context = ctx
  container.width = width
  container.height = height
  scheduleDraw(container)
}

const HostConfig = {
  rendererPackageName: 'pifp-stellar-canvas-renderer',
  rendererVersion: '0.1.0',
  extraDevToolsConfig: null,
  supportsMutation: true,
  supportsPersistence: false,
  supportsHydration: false,
  isPrimaryRenderer: false,
  warnsIfNotActing: false,
  supportsMicrotasks: true,
  supportsTestSelectors: false,
  now: Date.now,
  getRootHostContext() {
    return null
  },
  getChildHostContext() {
    return null
  },
  getPublicInstance(instance) {
    return instance
  },
  prepareForCommit() {
    return null
  },
  resetAfterCommit(container) {
    scheduleDraw(container)
  },
  preparePortalMount() {},
  scheduleTimeout: window.setTimeout.bind(window),
  cancelTimeout: window.clearTimeout.bind(window),
  noTimeout: -1,
  scheduleMicrotask: window.queueMicrotask.bind(window),
  getCurrentEventPriority() {
    return DefaultEventPriority
  },
  setCurrentUpdatePriority(newPriority) {
    currentUpdatePriority = newPriority
  },
  getCurrentUpdatePriority() {
    return currentUpdatePriority
  },
  resolveUpdatePriority() {
    return DefaultEventPriority
  },
  trackSchedulerEvent() {},
  resolveEventType() {
    return null
  },
  resolveEventTimeStamp() {
    return Date.now()
  },
  shouldAttemptEagerTransition() {
    return false
  },
  detachDeletedInstance() {},
  maySuspendCommit() {
    return false
  },
  maySuspendCommitOnUpdate() {
    return false
  },
  maySuspendCommitInSyncRender() {
    return false
  },
  preloadInstance() {
    return true
  },
  startSuspendingCommit() {},
  suspendInstance() {},
  waitForCommitToBeReady() {
    return null
  },
  getSuspendedCommitReason() {
    return null
  },
  NotPendingTransition: null,
  HostTransitionContext: null,
  resetFormInstance() {},
  bindToConsole(...args) {
    return Function.prototype.bind.call(console.log, console, ...args)
  },
  createInstance(type, props) {
    return {
      type,
      props: normalizeProps(props),
      children: [],
      parent: null,
      hidden: false,
    }
  },
  appendInitialChild(parent, child) {
    appendChildNode(parent, child)
  },
  finalizeInitialChildren() {
    return false
  },
  shouldSetTextContent() {
    return false
  },
  createTextInstance(text) {
    return {
      type: TEXT_INSTANCE,
      text,
      children: [],
      parent: null,
      hidden: false,
    }
  },
  appendChild(parent, child) {
    appendChildNode(parent, child)
  },
  appendChildToContainer(container, child) {
    appendChildNode(container, child)
  },
  insertBefore(parent, child, beforeChild) {
    insertChildNode(parent, child, beforeChild)
  },
  insertInContainerBefore(container, child, beforeChild) {
    insertChildNode(container, child, beforeChild)
  },
  removeChild(parent, child) {
    removeChildNode(parent, child)
  },
  removeChildFromContainer(container, child) {
    removeChildNode(container, child)
  },
  commitUpdate(instance, type, oldProps, newProps) {
    if (instance.type !== type || shallowDiffProps(oldProps, newProps)) {
      instance.props = normalizeProps(newProps)
    }
  },
  commitTextUpdate(textInstance, oldText, newText) {
    if (oldText !== newText) {
      textInstance.text = newText
    }
  },
  commitMount() {},
  resetTextContent(instance) {
    instance.children = instance.children.filter((child) => child.type !== TEXT_INSTANCE)
  },
  hideInstance(instance) {
    instance.hidden = true
  },
  hideTextInstance(instance) {
    instance.hidden = true
  },
  unhideInstance(instance) {
    instance.hidden = false
  },
  unhideTextInstance(instance) {
    instance.hidden = false
  },
  clearContainer(container) {
    container.children = []
    scheduleDraw(container)
    return false
  },
  prepareUpdate(instance, type, oldProps, newProps) {
    return instance.type !== type || shallowDiffProps(oldProps, newProps) ? true : null
  },
}

const CanvasReconciler = ReactReconciler(HostConfig)

export function createCanvasSurface(canvas, options = {}) {
  const container = {
    canvas: null,
    context: null,
    width: options.width ?? 960,
    height: options.height ?? 420,
    padding: options.padding ?? 28,
    background: options.background ?? '#08131a',
    frameId: null,
    children: [],
  }

  resizeSurface(container, canvas, container.width, container.height)

  const root = CanvasReconciler.createContainer(
    container,
    ConcurrentRoot,
    null,
    false,
    null,
    '',
    console.error,
    console.error,
    console.error,
    null,
  )

  return {
    container,
    root,
  }
}

export function resizeCanvasSurface(surface, width, height) {
  resizeSurface(surface.container, surface.container.canvas, width, height)
}

export function renderCanvasTree(surface, element) {
  CanvasReconciler.updateContainer(element, surface.root, null, null)
}

export function destroyCanvasSurface(surface) {
  if (surface.container.frameId !== null) {
    window.cancelAnimationFrame(surface.container.frameId)
    surface.container.frameId = null
  }

  CanvasReconciler.updateContainer(null, surface.root, null, null)
}
