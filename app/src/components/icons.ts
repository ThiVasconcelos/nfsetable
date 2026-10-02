// Inline SVG icon set (24x24 grid, drawn with strokes). Original drawings, bundled with the app.

const dot = (cx: number, cy: number) => `<circle cx="${cx}" cy="${cy}" r="1.4" fill="currentColor" stroke="none"/>`

const FILE = '<path d="M7 3.5h7l4.5 4.5v11a1.5 1.5 0 0 1-1.5 1.5H7a1.5 1.5 0 0 1-1.5-1.5v-14A1.5 1.5 0 0 1 7 3.5z"/><path d="M14 3.5V8h4.5"/>'
const FOLDER = '<path d="M3.5 7.5a2 2 0 0 1 2-2h3.8l2 2.5h7.2a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>'

export const ICONS = {
  folder: FOLDER,
  folderPlus: `${FOLDER}<path d="M12 11v5.5M9.25 13.75h5.5"/>`,
  folderOpen:
    '<path d="M3.5 17V7.5a2 2 0 0 1 2-2h3.8l2 2.5h6.2a2 2 0 0 1 2 2v.5"/><path d="M3.8 18.2 6.3 12a1.5 1.5 0 0 1 1.4-.9h12a1 1 0 0 1 .9 1.4l-2.4 5.6a1.5 1.5 0 0 1-1.4.9H5a1.4 1.4 0 0 1-1.2-.8z"/>',
  file: FILE,
  filePlus: `${FILE}<path d="M12 11.5v5.5M9.25 14.25h5.5"/>`,
  fileText: `${FILE}<path d="M9 12.5h6M9 16h4"/>`,
  drop: '<path d="M12 4v10M8 10l4 4 4-4"/><path d="M4.5 14.5v3a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-3"/>',
  x: '<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>',
  plus: '<path d="M12 5.5v13M5.5 12h13"/>',
  minus: '<path d="M5.5 12h13"/>',
  search: '<circle cx="11" cy="11" r="6.25"/><path d="M15.75 15.75 20 20"/>',
  copy: '<rect x="8.5" y="8.5" width="11" height="11" rx="2"/><path d="M15.5 8.5V6a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 6v8A1.5 1.5 0 0 0 6 15.5h2.5"/>',
  download: '<path d="M12 4v11M7.5 10.5 12 15l4.5-4.5"/><path d="M5 19.5h14"/>',
  sun: '<circle cx="12" cy="12" r="3.75"/><path d="M12 3v1.75M12 19.25V21M5.64 5.64l1.24 1.24M17.12 17.12l1.24 1.24M3 12h1.75M19.25 12H21M5.64 18.36l1.24-1.24M17.12 6.88l1.24-1.24"/>',
  moon: '<path d="M19.5 14.2A7.75 7.75 0 0 1 9.8 4.5a7.75 7.75 0 1 0 9.7 9.7z"/>',
  monitor: '<rect x="3" y="4.5" width="18" height="12" rx="2"/><path d="M8.5 20h7M12 16.5V20"/>',
  layers: '<path d="M12 4 3.5 8.25 12 12.5l8.5-4.25z"/><path d="m3.5 12.25 8.5 4.25 8.5-4.25"/><path d="m3.5 16.25 8.5 4.25 8.5-4.25"/>',
  chevronLeft: '<path d="M14.5 6 8.5 12l6 6"/>',
  chevronRight: '<path d="m9.5 6 6 6-6 6"/>',
  chevronDown: '<path d="m6 9.5 6 6 6-6"/>',
  zoomIn: '<circle cx="11" cy="11" r="6.25"/><path d="M15.75 15.75 20 20M11 8.5v5M8.5 11h5"/>',
  zoomOut: '<circle cx="11" cy="11" r="6.25"/><path d="M15.75 15.75 20 20M8.5 11h5"/>',
  fitWidth: '<path d="M4 5v14M20 5v14"/><path d="M7.5 12h9M10 9.5 7.5 12l2.5 2.5M14 9.5l2.5 2.5-2.5 2.5"/>',
  marquee:
    '<path d="M4 8V5.5A1.5 1.5 0 0 1 5.5 4H8M16 4h2.5A1.5 1.5 0 0 1 20 5.5V8M20 16v2.5a1.5 1.5 0 0 1-1.5 1.5H16M8 20H5.5A1.5 1.5 0 0 1 4 18.5V16"/><path d="M12 9v6M9 12h6"/>',
  more: dot(6, 12) + dot(12, 12) + dot(18, 12),
  externalLink:
    '<path d="M13.5 4.5h6v6M19.5 4.5l-8.25 8.25"/><path d="M18 14v4a1.5 1.5 0 0 1-1.5 1.5h-10A1.5 1.5 0 0 1 5 18V8a1.5 1.5 0 0 1 1.5-1.5H10"/>',
  trash:
    '<path d="M4.5 7h15M10 7V5a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v2M6.5 7l.8 11.6A1.5 1.5 0 0 0 8.8 20h6.4a1.5 1.5 0 0 0 1.5-1.4L17.5 7"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  checkCircle: '<circle cx="12" cy="12" r="8.5"/><path d="m8.25 12.25 2.5 2.5 5-5"/>',
  alert: '<path d="M10.3 4.9 3.2 17.3A2 2 0 0 0 4.9 20.3h14.2a2 2 0 0 0 1.7-3L13.7 4.9a2 2 0 0 0-3.4 0z"/><path d="M12 9.5v4.5M12 17v.25"/>',
  alertCircle: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.75v5M12 16v.25"/>',
  info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5M12 8v.25"/>',
  shield: '<path d="M12 3.5 5 6v5.5c0 4.2 3 7.6 7 9 4-1.4 7-4.8 7-9V6z"/><path d="m9 12 2.2 2.2L15.5 10"/>',
  pencil: '<path d="M14.8 5.7 18.3 9.2 8.5 19H5v-3.5z"/><path d="m12.8 7.7 3.5 3.5"/>',
  eyeOff:
    '<path d="M3.5 3.5l17 17"/><path d="M10.6 5.6A9 9 0 0 1 12 5.5c5 0 8.5 4.5 9.5 6.5-.5 1-1.6 2.6-3.2 4M6.5 6.9C4.6 8.2 3.2 10.2 2.5 12c1 2 4.5 6.5 9.5 6.5 1.6 0 3-.4 4.3-1.1"/><path d="M9.9 10a3 3 0 0 0 4.1 4.1"/>',
  refresh: '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>',
  terminal: '<rect x="3" y="4.5" width="18" height="15" rx="2"/><path d="m7 9.5 3 2.5-3 2.5M12.5 15H17"/>',
  lock: '<rect x="5" y="10.5" width="14" height="9.5" rx="2"/><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"/>',
  panelOpen: '<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><path d="M9 4.5v15"/><path d="m13.25 10 2 2-2 2"/>',
  panelClose: '<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><path d="M9 4.5v15"/><path d="m15.25 10-2 2 2 2"/>',
  chevronUp: '<path d="m6 14.5 6-6 6 6"/>',
  arrowUp: '<path d="M12 19V5.5M6.75 10.75 12 5.5l5.25 5.25"/>',
  arrowDown: '<path d="M12 5v13.5M6.75 13.25 12 18.5l5.25-5.25"/>',
  arrowUpDown: '<path d="M8.5 18.5v-13M5.5 8.5l3-3 3 3M15.5 5.5v13M12.5 15.5l3 3 3-3"/>',
  table: '<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><path d="M3.5 9.5h17M3.5 14.5h17M9.5 9.5v10"/>',
  calculator:
    '<rect x="5" y="3.5" width="14" height="17" rx="2.25"/><path d="M8.5 7.25h7"/><circle cx="9" cy="11.5" r="1" fill="currentColor" stroke="none"/><circle cx="12" cy="11.5" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="11.5" r="1" fill="currentColor" stroke="none"/><circle cx="9" cy="15" r="1" fill="currentColor" stroke="none"/><circle cx="12" cy="15" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="15" r="1" fill="currentColor" stroke="none"/>',
  calendar: '<rect x="4" y="5.5" width="16" height="14.5" rx="2"/><path d="M4 10h16M8.5 3.5v4M15.5 3.5v4"/>',
  tag: `<path d="M3.75 12.1V5a1.25 1.25 0 0 1 1.25-1.25h7.1a1.2 1.2 0 0 1 .85.35l7.4 7.4a1.25 1.25 0 0 1 0 1.77l-7.1 7.1a1.25 1.25 0 0 1-1.77 0l-7.4-7.4a1.2 1.2 0 0 1-.33-.87z"/>${dot(8.25, 8.25)}`,
  fileGrid: `${FILE}<path d="M8.5 12h7M8.5 15h7M8.5 18h4"/>`,
  sheet: `${FILE}<path d="M8.5 11.5h7v6.5h-7zM8.5 14.75h7M12 11.5V18"/>`,
  database:
    '<ellipse cx="12" cy="6" rx="6.75" ry="2.5"/><path d="M5.25 6v12c0 1.38 3.02 2.5 6.75 2.5s6.75-1.12 6.75-2.5V6"/><path d="M5.25 12c0 1.38 3.02 2.5 6.75 2.5s6.75-1.12 6.75-2.5"/>',
  wallet:
    '<path d="M4 7.75A2.75 2.75 0 0 1 6.75 5h10.5A1.25 1.25 0 0 1 18.5 6.25V8"/><rect x="4" y="8" width="16.5" height="11.5" rx="2"/><path d="M15.75 13.75h1.5"/>',
  target: '<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="4.25"/><circle cx="12" cy="12" r="1" fill="currentColor" stroke="none"/>',
  book: '<path d="M5 5.75A1.75 1.75 0 0 1 6.75 4H19v13.5H6.75A1.75 1.75 0 0 0 5 19.25z"/><path d="M5 19.25A1.75 1.75 0 0 0 6.75 21H19"/>',
  barChart: '<path d="M4.5 19.5h15"/><path d="M7.5 16v-4.5M12 16V7.5M16.5 16v-6.5"/>',
  building:
    '<path d="M4.5 20.5h15M6.5 20.5V5.25a.75.75 0 0 1 .75-.75h6.5a.75.75 0 0 1 .75.75V20.5M14.5 9.5h3.25a.75.75 0 0 1 .75.75V20.5"/><path d="M9 8.5h2.5M9 12h2.5M9 15.5h2.5"/>',
  user: '<circle cx="12" cy="8.5" r="3.75"/><path d="M5 20a7 7 0 0 1 14 0"/>',
  scale:
    '<path d="M12 4.5v15M7.5 19.5h9M5.5 7.5h13"/><path d="M5.5 7.5 3 13.25a2.6 2.6 0 0 0 5 0zM18.5 7.5 16 13.25a2.6 2.6 0 0 0 5 0z"/>',
  coins:
    '<ellipse cx="9.5" cy="7" rx="5" ry="2.5"/><path d="M4.5 7v4c0 1.38 2.24 2.5 5 2.5s5-1.12 5-2.5V7"/><path d="M4.5 11v4c0 1.38 2.24 2.5 5 2.5 1.1 0 2.1-.18 2.93-.48M14.5 11.6c.3-.07.63-.1.97-.1 2.76 0 5 1.12 5 2.5v3.5c0 1.38-2.24 2.5-5 2.5s-5-1.12-5-2.5V14"/>',
  settings:
    '<path d="M10.3 3.9a1 1 0 0 1 1-.9h1.4a1 1 0 0 1 1 .9l.2 1.6a6.5 6.5 0 0 1 1.7 1l1.5-.6a1 1 0 0 1 1.2.4l.7 1.2a1 1 0 0 1-.2 1.3l-1.3 1a6.5 6.5 0 0 1 0 2l1.3 1a1 1 0 0 1 .2 1.3l-.7 1.2a1 1 0 0 1-1.2.4l-1.5-.6a6.5 6.5 0 0 1-1.7 1l-.2 1.6a1 1 0 0 1-1 .9h-1.4a1 1 0 0 1-1-.9l-.2-1.6a6.5 6.5 0 0 1-1.7-1l-1.5.6a1 1 0 0 1-1.2-.4l-.7-1.2a1 1 0 0 1 .2-1.3l1.3-1a6.5 6.5 0 0 1 0-2l-1.3-1a1 1 0 0 1-.2-1.3l.7-1.2a1 1 0 0 1 1.2-.4l1.5.6a6.5 6.5 0 0 1 1.7-1z"/><circle cx="12" cy="12" r="2.75"/>',
  link: '<path d="M10 14a4 4 0 0 0 5.66 0l2.83-2.83a4 4 0 0 0-5.66-5.66L11.5 6.84"/><path d="M14 10a4 4 0 0 0-5.66 0L5.5 12.83a4 4 0 0 0 5.66 5.66l1.33-1.33"/>',
  hardDrive: `<rect x="3.5" y="13" width="17" height="6.5" rx="2"/><path d="M5.5 13 8 5.5h8l2.5 7.5"/>${dot(16.5, 16.25)}`,
  receipt: '<path d="M6 3.5h12v17l-2-1.25-2 1.25-2-1.25-2 1.25-2-1.25-2 1.25z"/><path d="M9 8h6M9 11.5h6M9 15h3.5"/>',
  wand: '<path d="m4.5 19.5 10-10M13 5.5l1-2 1 2 2 1-2 1-1 2-1-2-2-1zM18.5 12l.6-1.2.6 1.2 1.2.6-1.2.6-.6 1.2-.6-1.2-1.2-.6z"/><path d="m13 8 3 3"/>',
} as const

export type IconName = keyof typeof ICONS
