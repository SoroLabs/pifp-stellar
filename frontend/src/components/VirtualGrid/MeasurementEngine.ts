/**
 * MeasurementEngine caches heights of items and computes positions asynchronously.
 * It prevents reflows by keeping track of the sizes without forcing synchronous layouts.
 */

export class MeasurementEngine {
  private heights: number[] = [];
  private positions: number[] = [];
  private totalHeight: number = 0;
  private defaultHeight: number;
  private listeners: (() => void)[] = [];

  constructor(defaultHeight: number = 60) {
    this.defaultHeight = defaultHeight;
  }

  public subscribe(listener: () => void) {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify() {
    this.listeners.forEach((l) => l());
  }

  public setItemCount(count: number) {
    if (this.heights.length < count) {
      const oldLen = this.heights.length;
      for (let i = oldLen; i < count; i++) {
        this.heights[i] = this.defaultHeight;
      }
      this.recalculatePositions(oldLen);
      this.notify();
    }
  }

  public measureItem(index: number, height: number) {
    if (this.heights[index] !== height) {
      this.heights[index] = height;
      this.recalculatePositions(index);
      this.notify();
    }
  }

  private recalculatePositions(startIndex: number) {
    for (let i = startIndex; i < this.heights.length; i++) {
      if (i === 0) {
        this.positions[i] = 0;
      } else {
        this.positions[i] = this.positions[i - 1] + this.heights[i - 1];
      }
    }
    const lastIdx = this.heights.length - 1;
    this.totalHeight = lastIdx >= 0 ? this.positions[lastIdx] + this.heights[lastIdx] : 0;
  }

  public getTotalHeight() {
    return this.totalHeight;
  }

  public getHeight(index: number) {
    return this.heights[index] || this.defaultHeight;
  }

  public getPosition(index: number) {
    return this.positions[index] || 0;
  }

  public getVisibleRange(scrollTop: number, viewportHeight: number): { startIndex: number; endIndex: number } {
    let startIndex = 0;
    // Binary search for the first visible item
    let low = 0;
    let high = this.positions.length - 1;

    while (low <= high) {
      const mid = Math.floor((low + high) / 2);
      if (this.positions[mid] <= scrollTop) {
        startIndex = mid;
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }

    let endIndex = startIndex;
    let currentPos = this.positions[startIndex];
    
    // Find the end index
    while (endIndex < this.positions.length && currentPos < scrollTop + viewportHeight) {
      currentPos += this.heights[endIndex];
      endIndex++;
    }

    return { startIndex, endIndex: Math.min(endIndex, this.positions.length - 1) };
  }
}
