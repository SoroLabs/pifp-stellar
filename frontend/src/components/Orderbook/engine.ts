import { SkipList } from './SkipList';

export interface OrderUpdate {
  price: number;
  size: number; // 0 means remove
}

export class OrderbookEngine {
  private skipList: SkipList;
  private rowMap: Map<number, HTMLTableRowElement>;
  private container: HTMLTableSectionElement;

  constructor(containerId: string) {
    this.skipList = new SkipList();
    this.rowMap = new Map();
    const el = document.getElementById(containerId);
    if (!el) throw new Error(`Container ${containerId} not found`);
    this.container = el as HTMLTableSectionElement;
  }

  public processUpdates(updates: OrderUpdate[]) {
    // Process updates in memory
    for (const update of updates) {
      if (update.size === 0) {
        this.skipList.delete(update.price);
        this.removeRow(update.price);
      } else {
        const existing = this.skipList.get(update.price);
        let sizeIncreased = true;
        if (existing) {
          sizeIncreased = update.size > existing.size;
        }
        this.skipList.insert(update.price, update.size);
        this.updateOrInsertRow(update.price, update.size, sizeIncreased);
      }
    }

    // After processing, ensure DOM order matches SkipList order
    this.reorderDOM();
  }

  private removeRow(price: number) {
    const row = this.rowMap.get(price);
    if (row) {
      // Add red flash then remove
      row.classList.add('flash-red');
      setTimeout(() => {
        if (row.parentNode === this.container) {
          this.container.removeChild(row);
        }
        this.rowMap.delete(price);
      }, 300); // Wait for animation
    }
  }

  private updateOrInsertRow(price: number, size: number, sizeIncreased: boolean) {
    let row = this.rowMap.get(price);
    if (!row) {
      row = document.createElement('tr');
      row.dataset.price = price.toString();
      
      const priceCell = document.createElement('td');
      priceCell.textContent = price.toFixed(2);
      priceCell.className = 'price-cell';

      const sizeCell = document.createElement('td');
      sizeCell.textContent = size.toString();
      sizeCell.className = 'size-cell';

      row.appendChild(priceCell);
      row.appendChild(sizeCell);

      this.rowMap.set(price, row);
      this.container.appendChild(row); // Will be sorted in reorderDOM
      
      this.triggerFlash(row, 'flash-green');
    } else {
      const sizeCell = row.querySelector('.size-cell');
      if (sizeCell && sizeCell.textContent !== size.toString()) {
        sizeCell.textContent = size.toString();
        this.triggerFlash(row, sizeIncreased ? 'flash-green' : 'flash-red');
      }
    }
  }

  private triggerFlash(element: HTMLElement, className: string) {
    // Reset animation
    element.classList.remove('flash-green', 'flash-red');
    // Force reflow to restart animation
    void element.offsetWidth;
    element.classList.add(className);
  }

  private reorderDOM() {
    // For a highly optimized orderbook, we would only move nodes that are out of order.
    // However, Node.appendChild on an existing node moves it without full recreation.
    let currentDOMNode = this.container.firstElementChild;
    for (const node of this.skipList.entries()) {
      const row = this.rowMap.get(node.price);
      if (row) {
        if (currentDOMNode !== row) {
          this.container.insertBefore(row, currentDOMNode);
        } else {
          currentDOMNode = currentDOMNode.nextElementSibling;
        }
      }
    }
  }

  public destroy() {
    this.container.innerHTML = '';
    this.rowMap.clear();
  }
}
