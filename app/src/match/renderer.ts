import type { Pitch, Side } from '../api';
import type { Sample } from './timeline';

/** What a renderer needs to know about the match before drawing it. */
export interface MatchScene {
  pitch: Pitch;
  /** Kit colours by side: [primary, secondary]. */
  kits: Record<Side, [string, string]>;
  /** Shirt numbers (or initials) by player id. */
  labels: Map<number, string>;
}

/** Per-draw extras. */
export interface Overlay {
  /** A player to highlight. */
  selected?: number | null;
}

/**
 * Draws a match from samples of the frame stream. The 2D canvas is one
 * implementation; a WebGL or 3D view implements the same interface and
 * the match screen does not change.
 */
export interface MatchRenderer {
  mount(container: HTMLElement): void;
  setScene(scene: MatchScene): void;
  draw(sample: Sample, overlay?: Overlay): void;
  /** Call when the container changes size. */
  resize(): void;
  destroy(): void;
}
