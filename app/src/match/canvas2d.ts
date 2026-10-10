import type { MatchRenderer, MatchScene, Overlay } from './renderer';
import type { Sample } from './timeline';

/** Metres of grass drawn around the pitch. */
const MARGIN = 4;

/** A top-down 2D view on a canvas. */
export class Canvas2DRenderer implements MatchRenderer {
  private canvas = document.createElement('canvas');
  private ctx = this.canvas.getContext('2d')!;
  private scene: MatchScene | null = null;
  private container: HTMLElement | null = null;
  private scale = 1;
  private dpr = 1;
  private observer: ResizeObserver | null = null;

  mount(container: HTMLElement): void {
    this.container = container;
    this.canvas.style.display = 'block';
    this.canvas.style.width = '100%';
    this.canvas.style.height = '100%';
    this.canvas.setAttribute('role', 'img');
    this.canvas.setAttribute('aria-label', 'Match view');
    container.appendChild(this.canvas);
    this.observer = new ResizeObserver(() => this.resize());
    this.observer.observe(container);
    this.resize();
  }

  setScene(scene: MatchScene): void {
    this.scene = scene;
    this.resize();
  }

  resize(): void {
    if (!this.container || !this.scene) return;
    const { length, width } = this.scene.pitch;
    const w = this.container.clientWidth;
    const h = this.container.clientHeight;
    const totalW = length + 2 * MARGIN;
    const totalH = width + 2 * MARGIN;
    this.scale = Math.max(1, Math.min(w / totalW, h / totalH));
    this.dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.round(totalW * this.scale * this.dpr);
    this.canvas.height = Math.round(totalH * this.scale * this.dpr);
    this.canvas.style.width = `${totalW * this.scale}px`;
    this.canvas.style.height = `${totalH * this.scale}px`;
  }

  destroy(): void {
    this.observer?.disconnect();
    this.canvas.remove();
  }

  draw(sample: Sample, overlay: Overlay = {}): void {
    const scene = this.scene;
    if (!scene) return;
    const c = this.ctx;
    const s = this.scale * this.dpr;
    c.setTransform(s, 0, 0, s, MARGIN * s, MARGIN * s);
    this.drawPitch(scene);

    // Players, then the ball on top.
    for (const p of sample.players) {
      const [fill, trim] = scene.kits[p.side];
      const r = 1.25;
      // Shadow.
      c.fillStyle = 'rgba(0,0,0,0.25)';
      c.beginPath();
      c.ellipse(p.x + 0.3, p.y + 0.4, r, r * 0.8, 0, 0, Math.PI * 2);
      c.fill();
      // Body.
      c.fillStyle = fill;
      c.strokeStyle = trim;
      c.lineWidth = 0.35;
      c.beginPath();
      c.arc(p.x, p.y, r, 0, Math.PI * 2);
      c.fill();
      c.stroke();
      // Facing: a tick outside the shirt.
      c.strokeStyle = trim;
      c.lineWidth = 0.3;
      c.beginPath();
      c.moveTo(p.x + Math.cos(p.facing) * r * 1.05, p.y + Math.sin(p.facing) * r * 1.05);
      c.lineTo(p.x + Math.cos(p.facing) * r * 1.7, p.y + Math.sin(p.facing) * r * 1.7);
      c.stroke();
      if (p.pose === 'celebrating' || p.pose === 'shooting' || p.pose === 'diving' || p.pose === 'tackling') {
        c.strokeStyle = p.pose === 'celebrating' ? '#ffd54a' : 'rgba(255,255,255,0.8)';
        c.lineWidth = 0.25;
        c.beginPath();
        c.arc(p.x, p.y, r + 0.6, 0, Math.PI * 2);
        c.stroke();
      }
      if (overlay.selected === p.id) {
        c.strokeStyle = '#3ddc84';
        c.lineWidth = 0.4;
        c.beginPath();
        c.arc(p.x, p.y, r + 1, 0, Math.PI * 2);
        c.stroke();
      }
      const label = scene.labels.get(p.id);
      if (label) {
        c.fillStyle = trim;
        c.font = `700 ${r * 1.15}px system-ui, sans-serif`;
        c.textAlign = 'center';
        c.textBaseline = 'middle';
        c.fillText(label, p.x, p.y + 0.05);
      }
    }

    const [bx, by, bz] = sample.ball;
    c.fillStyle = 'rgba(0,0,0,0.3)';
    c.beginPath();
    c.ellipse(bx + bz * 0.25, by + bz * 0.35, 0.45, 0.35, 0, 0, Math.PI * 2);
    c.fill();
    const br = 0.45 + Math.min(bz, 12) * 0.04;
    c.fillStyle = '#ffffff';
    c.strokeStyle = '#111';
    c.lineWidth = 0.12;
    c.beginPath();
    c.arc(bx, by, br, 0, Math.PI * 2);
    c.fill();
    c.stroke();
  }

  private drawPitch(scene: MatchScene): void {
    const c = this.ctx;
    const { length: L, width: W } = scene.pitch;
    // Grass with mowing stripes.
    c.fillStyle = '#1c6335';
    c.fillRect(-MARGIN, -MARGIN, L + 2 * MARGIN, W + 2 * MARGIN);
    const stripes = 14;
    for (let i = 0; i < stripes; i += 2) {
      c.fillStyle = '#1f6b3a';
      c.fillRect((i * L) / stripes, 0, L / stripes, W);
    }
    c.strokeStyle = 'rgba(255,255,255,0.75)';
    c.lineWidth = 0.15;
    c.strokeRect(0, 0, L, W);
    // Halfway line and centre circle.
    c.beginPath();
    c.moveTo(L / 2, 0);
    c.lineTo(L / 2, W);
    c.stroke();
    c.beginPath();
    c.arc(L / 2, W / 2, 9.15, 0, Math.PI * 2);
    c.stroke();
    this.spot(L / 2, W / 2);
    for (const end of [0, 1]) {
      const dir = end === 0 ? 1 : -1;
      const x0 = end === 0 ? 0 : L;
      // Penalty area, goal area, spot and arc.
      c.strokeRect(Math.min(x0, x0 + dir * 16.5), W / 2 - 20.16, 16.5, 40.32);
      c.strokeRect(Math.min(x0, x0 + dir * 5.5), W / 2 - 9.16, 5.5, 18.32);
      const spot = x0 + dir * 11;
      this.spot(spot, W / 2);
      const a = Math.acos(5.5 / 9.15);
      c.beginPath();
      if (end === 0) c.arc(spot, W / 2, 9.15, -a, a);
      else c.arc(spot, W / 2, 9.15, Math.PI - a, Math.PI + a);
      c.stroke();
      // Goal.
      c.fillStyle = 'rgba(255,255,255,0.25)';
      c.fillRect(end === 0 ? -2 : L, W / 2 - 3.66, 2, 7.32);
      c.strokeRect(end === 0 ? -2 : L, W / 2 - 3.66, 2, 7.32);
    }
  }

  private spot(x: number, y: number): void {
    const c = this.ctx;
    c.fillStyle = 'rgba(255,255,255,0.75)';
    c.beginPath();
    c.arc(x, y, 0.25, 0, Math.PI * 2);
    c.fill();
  }
}

/** Kits for both sides, changing the away kit if the colours clash. */
export function kits(home: [string, string], away: [string, string]): { home: [string, string]; away: [string, string] } {
  const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  const dist = (a: string, b: string) => {
    const [x, y] = [rgb(a), rgb(b)];
    return Math.hypot(x[0] - y[0], x[1] - y[1], x[2] - y[2]);
  };
  if (dist(home[0], away[0]) > 120) return { home, away };
  if (dist(home[0], away[1]) > 120) return { home, away: [away[1], away[0]] };
  return { home, away: dist(home[0], '#ffffff') > 120 ? ['#ffffff', '#222222'] : ['#222222', '#ffffff'] };
}
