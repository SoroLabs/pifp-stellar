class PifpMasonryLayout {
  static get inputProperties() {
    return ['--masonry-gap', '--masonry-columns']
  }

  async intrinsicSizes(children, edges, constraints) {
    const columnCount = Math.max(1, Math.min(5, Math.floor((constraints.fixedInlineSize || 960) / 280)))
    const columnWidth = Math.floor((constraints.fixedInlineSize || 960 - (columnCount - 1) * 16) / columnCount)
    let totalBlockSize = 0
    for (const child of children) {
      const fragment = child.layoutNextFragment({ availableInlineSize: columnWidth, availableBlockSize: constraints.availableBlockSize })
      totalBlockSize = Math.max(totalBlockSize, fragment.logicalHeight || 0)
    }
    return { autoInlineSize: constraints.fixedInlineSize || 960, autoBlockSize: totalBlockSize * Math.ceil(children.length / columnCount) }
  }

  async layout(children, edges, constraints, styleMap) {
    const containerWidth = constraints.fixedInlineSize || constraints.availableInlineSize || 960
    const gap = parseInt(styleMap.get('--masonry-gap')?.value || 16, 10)
    const columns = Math.max(1, Math.min(5, Math.floor(containerWidth / 280)))
    const columnWidth = Math.floor((containerWidth - gap * (columns - 1)) / columns)
    const columnHeights = new Array(columns).fill(0)
    const childFragments = []

    for (const child of children) {
      const fragment = child.layoutNextFragment({ availableInlineSize: columnWidth, availableBlockSize: constraints.availableBlockSize })
      childFragments.push(fragment)
    }

    const childPositions = childFragments.map((fragment) => {
      const index = columnHeights.indexOf(Math.min(...columnHeights))
      const x = index * (columnWidth + gap)
      const y = columnHeights[index]
      columnHeights[index] += (fragment.logicalHeight || 0) + gap
      return { x, y, width: columnWidth, height: fragment.logicalHeight || 0 }
    })

    const containerHeight = Math.max(...columnHeights) - gap
    return { autoBlockSize: containerHeight, childFragments: childFragments.map((fragment, idx) => ({ fragment, inlineOffset: childPositions[idx].x, blockOffset: childPositions[idx].y })) }
  }
}

registerLayout('pifp-masonry', PifpMasonryLayout)
