export type ScrollContainer = { scrollTop: number };

/** Remembers each page's scroll offset so leaving and returning does not jump to the top. */
export class ScrollMemory {
  private offsets = new Map<string, number>();

  save(id: string, scroller: ScrollContainer | null | undefined) {
    if (!scroller) return;
    this.offsets.set(id, scroller.scrollTop);
  }

  apply(id: string, scroller: ScrollContainer | null | undefined) {
    if (!scroller) return;
    scroller.scrollTop = this.offsets.get(id) ?? 0;
  }
}
