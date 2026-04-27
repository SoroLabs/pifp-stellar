class PifpCardBackgroundPainter {
  static get inputProperties() {
    return ['--card-hue']
  }

  paint(ctx, geom, properties) {
    const hue = Number(properties.get('--card-hue')?.toString() || 200)
    const width = geom.width
    const height = geom.height
    const gradient = ctx.createLinearGradient(0, 0, width, height)
    gradient.addColorStop(0, `hsl(${hue * 25 % 360}, 70%, 88%)`)
    gradient.addColorStop(0.5, `hsl(${(hue * 25 + 45) % 360}, 72%, 77%)`)
    gradient.addColorStop(1, `hsl(${(hue * 25 + 90) % 360}, 65%, 85%)`)
    ctx.fillStyle = gradient
    ctx.fillRect(0, 0, width, height)

    ctx.fillStyle = 'rgba(255,255,255,0.22)'
    for (let i = 0; i < 6; i += 1) {
      ctx.beginPath()
      ctx.arc(width * (0.12 + (i * 0.14)), height * (0.25 + (i % 2) * 0.12), width * 0.16, 0, Math.PI * 2)
      ctx.fill()
    }

    ctx.fillStyle = 'rgba(15, 36, 48, 0.09)'
    for (let i = 0; i < 4; i += 1) {
      ctx.fillRect(width * 0.05, height * (0.3 + 0.16 * i), width * 0.9, 6)
    }
  }
}

registerPaint('pifp-card-bg', PifpCardBackgroundPainter)
