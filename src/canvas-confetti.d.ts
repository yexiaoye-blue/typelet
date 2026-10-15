declare module 'canvas-confetti' {
  interface Options {
    particleCount?: number;
    angle?: number;
    spread?: number;
    startVelocity?: number;
    decay?: number;
    gravity?: number;
    drift?: number;
    ticks?: number;
    origin?: { x?: number; y?: number };
    scalar?: number;
    colors?: string[];
    disableForReducedMotion?: boolean;
  }

  interface GlobalOptions {
    resize?: boolean;
    useWorker?: boolean;
    disableForReducedMotion?: boolean;
  }

  interface Create {
    (options?: Options): Promise<null> | null;
  }

  interface Confetti {
    (options?: Options): Promise<null> | null;
    create(canvas: HTMLCanvasElement, options?: GlobalOptions): Create;
  }

  const confetti: Confetti;
  export default confetti;
}
