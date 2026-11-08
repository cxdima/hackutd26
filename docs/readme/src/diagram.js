// Tiny absolute-layout diagram kit for README figures (rendered by ../render.mjs).
// Boxes are positioned explicitly so connectors can be routed by hand.
(() => {
function mount(frame, { width, height }) {
  frame.style.width = `${width}px`;
  frame.style.height = `${height}px`;
  frame.classList.add('grid-bg');
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('width', width);
  svg.setAttribute('height', height);
  svg.style.cssText = 'position:absolute;inset:0;pointer-events:none;z-index:3';
  svg.innerHTML = `<defs>
    ${['lime', 'water', 'amber', 'paper', 'magenta'].map((c) => `
    <marker id="arrow-${c}" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0,0 L10,5 L0,10 z" fill="var(--c-${c})"/>
    </marker>`).join('')}
  </defs>`;
  frame.appendChild(svg);
  return { frame, svg };
}

function el(frame, html, style, cls = '') {
  const d = document.createElement('div');
  d.className = cls;
  d.style.cssText = `position:absolute;${style}`;
  d.innerHTML = html;
  frame.appendChild(d);
  return d;
}

function group(frame, { x, y, w, h, label, tone = 'paper' }) {
  return el(frame, `<span class="g-label slate tone-${tone}">${label}</span>`,
    `left:${x}px;top:${y}px;width:${w}px;height:${h}px`, `group tone-${tone}`);
}

function box(frame, { x, y, w, h, title, tag, file, lines = [], tone = 'core', badge, html }) {
  const body = html ?? lines.map((l) => `<div class="ln">${l}</div>`).join('');
  return el(frame, `
    <div class="b-head">
      <span class="b-title">${title}</span>
      ${badge ? `<span class="badge ${badge.tone ?? 'amber'}">${badge.text}</span>` : ''}
      ${tag ? `<span class="b-tag">${tag}</span>` : ''}
    </div>
    ${file ? `<div class="b-file mono">${file}</div>` : ''}
    <div class="b-body mono">${body}</div>`,
    `left:${x}px;top:${y}px;width:${w}px;height:${h}px`, `box tone-${tone}`);
}

// points: [[x,y], ...] orthogonal polyline. opts: color, dashed, start/end arrows, num
function wire({ svg }, points, { color = 'paper', dashed = false, start = false, end = true, num, numAt, width = 1.6 } = {}) {
  const d = points.map(([x, y], i) => `${i ? 'L' : 'M'}${x},${y}`).join(' ');
  const p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  p.setAttribute('d', d);
  p.setAttribute('fill', 'none');
  p.setAttribute('stroke', `var(--c-${color})`);
  p.setAttribute('stroke-width', width);
  p.setAttribute('stroke-linejoin', 'round');
  if (dashed) p.setAttribute('stroke-dasharray', '5 4');
  if (end) p.setAttribute('marker-end', `url(#arrow-${color})`);
  if (start) p.setAttribute('marker-start', `url(#arrow-${color})`);
  svg.appendChild(p);
  if (num) {
    const [cx, cy] = numAt ?? points[Math.floor(points.length / 2)];
    const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    g.innerHTML = `<circle cx="${cx}" cy="${cy}" r="10" fill="var(--ink)" stroke="var(--c-${color})" stroke-width="1.5"/>
      <text x="${cx}" y="${cy + 4}" text-anchor="middle" font-family="Manrope" font-weight="800" font-size="11.5" fill="var(--c-${color})">${num}</text>`;
    svg.appendChild(g);
  }
  return p;
}

function header(frame, { n, label, title, sub }) {
  return el(frame, `
    <div class="eyebrow slate"><i></i>${n} · ${label}</div>
    <div class="d-title">${title}</div>
    ${sub ? `<div class="d-sub">${sub}</div>` : ''}`, 'left:32px;top:28px;right:32px', 'd-header');
}

// Classic script (file:// pages cannot import ES modules).
window.D = { mount, el, group, box, wire, header };
})();
