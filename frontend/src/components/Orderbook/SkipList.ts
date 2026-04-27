export class Node {
  price: number;
  size: number;
  forward: Node[];

  constructor(price: number, size: number, level: number) {
    this.price = price;
    this.size = size;
    this.forward = new Array(level + 1).fill(null);
  }
}

export class SkipList {
  private MAX_LEVEL = 16;
  private P = 0.5;
  public header: Node;
  public level: number;

  constructor() {
    this.header = new Node(-1, 0, this.MAX_LEVEL);
    this.level = 0;
  }

  private randomLevel(): number {
    let lvl = 0;
    while (Math.random() < this.P && lvl < this.MAX_LEVEL) {
      lvl++;
    }
    return lvl;
  }

  insert(price: number, size: number): void {
    const update = new Array(this.MAX_LEVEL + 1).fill(null);
    let current = this.header;

    for (let i = this.level; i >= 0; i--) {
      while (current.forward[i] && current.forward[i].price < price) {
        current = current.forward[i];
      }
      update[i] = current;
    }

    current = current.forward[0];

    // If price exists, just update size
    if (current && current.price === price) {
      current.size = size;
      return;
    }

    const lvl = this.randomLevel();
    if (lvl > this.level) {
      for (let i = this.level + 1; i <= lvl; i++) {
        update[i] = this.header;
      }
      this.level = lvl;
    }

    const newNode = new Node(price, size, lvl);
    for (let i = 0; i <= lvl; i++) {
      newNode.forward[i] = update[i].forward[i];
      update[i].forward[i] = newNode;
    }
  }

  delete(price: number): void {
    const update = new Array(this.MAX_LEVEL + 1).fill(null);
    let current = this.header;

    for (let i = this.level; i >= 0; i--) {
      while (current.forward[i] && current.forward[i].price < price) {
        current = current.forward[i];
      }
      update[i] = current;
    }

    current = current.forward[0];

    if (current && current.price === price) {
      for (let i = 0; i <= this.level; i++) {
        if (update[i].forward[i] !== current) {
          break;
        }
        update[i].forward[i] = current.forward[i];
      }

      while (this.level > 0 && this.header.forward[this.level] === null) {
        this.level--;
      }
    }
  }

  get(price: number): Node | null {
    let current = this.header;
    for (let i = this.level; i >= 0; i--) {
      while (current.forward[i] && current.forward[i].price < price) {
        current = current.forward[i];
      }
    }
    current = current.forward[0];
    if (current && current.price === price) {
      return current;
    }
    return null;
  }

  // Iterate over all nodes in order
  *entries(): IterableIterator<Node> {
    let current = this.header.forward[0];
    while (current) {
      yield current;
      current = current.forward[0];
    }
  }
}
