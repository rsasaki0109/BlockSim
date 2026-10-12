import type { Comparison, Sample } from './types';
const NS = 'http://www.w3.org/2000/svg';
function element<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string>) {
  const el = document.createElementNS(NS, tag);
  for (const [key, value] of Object.entries(attrs)) el.setAttribute(key, value);
  return el;
}
export function drawChart(host: HTMLElement, result: Comparison, field: 'output' | 'control' | 'integral', selected: boolean) {
  const series = [
    { name: 'AW 有効', data: result.enabled.samples, color: '#63e3bc', key: field },
    { name: 'AW 無効', data: result.disabled.samples, color: '#ffa96b', key: field },
    ...(field === 'output' ? [{ name: '目標値', data: result.enabled.samples, color: '#9facc5', key: 'reference' as const }] : []),
  ];
  let lo = 0, hi = 0;
  for (const s of series) for (const p of s.data) {
    lo = Math.min(lo, p[s.key]); hi = Math.max(hi, p[s.key]);
  }
  const span = hi - lo || 1;
  lo -= span * 0.08; hi += span * 0.08;
  const width = 900, height = 240, left = 58, top = 15, right = 16, bottom = 34;
  const duration = result.enabled.samples.at(-1)!.time;
  const x = (t: number) => left + t / duration * (width - left - right);
  const y = (v: number) => height - bottom - (v-lo)/(hi-lo)*(height-top-bottom);
  const svg = element('svg', { viewBox: `0 0 ${width} ${height}`, role: 'img', 'aria-label': host.dataset.title || field });
  for (let i = 0; i <= 4; i++) {
    const value = lo + (hi-lo)*i/4, py = y(value);
    svg.append(element('line', { x1: `${left}`, x2: `${width-right}`, y1: `${py}`, y2: `${py}`, stroke: '#29374d' }));
    const label = element('text', { x: `${left-9}`, y: `${py+4}`, 'text-anchor': 'end', fill: '#9facc5', 'font-size': '12' });
    label.textContent = value.toFixed(2); svg.append(label);
  }
  for (let i = 0; i <= 5; i++) {
    const t = duration*i/5;
    const label = element('text', { x: `${x(t)}`, y: `${height-10}`, 'text-anchor': 'middle', fill: '#9facc5', 'font-size': '12' });
    label.textContent = `${t.toFixed(1)}s`; svg.append(label);
  }
  series.forEach((s, index) => {
    // Bound rendering cost while preserving the final point. Metrics use all data.
    const stride = Math.max(1, Math.ceil(s.data.length/2000));
    const points: Sample[] = s.data.filter((_, i) => i%stride === 0);
    if (points.at(-1) !== s.data.at(-1)) points.push(s.data.at(-1)!);
    const d = points.map((p, i) => `${i ? 'L' : 'M'}${x(p.time).toFixed(2)},${y(p[s.key]).toFixed(2)}`).join(' ');
    svg.append(element('path', { d, fill: 'none', stroke: s.color,
      'stroke-width': index === (selected ? 0 : 1) ? '3' : '2',
      ...(index === 2 ? { 'stroke-dasharray': '7 5' } : {}), 'data-series': s.name }));
  });
  host.replaceChildren(svg);
}
