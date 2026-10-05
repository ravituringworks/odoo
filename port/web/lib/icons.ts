// Thin line icons (VibeCody iconography: 24px grid, round caps/joins, stroke from --icon-stroke = 1.5, currentColor).
// Paths are plain `d` strings; helpers below turn circles and rounded rects into paths so the renderer needs only <path>.
const c = (x: number, y: number, r: number) => `M${x - r} ${y}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`
const r = (x: number, y: number, w: number, h: number, k = 2) => `M${x + k} ${y}h${w - 2 * k}a${k} ${k} 0 0 1 ${k} ${k}v${h - 2 * k}a${k} ${k} 0 0 1 ${-k} ${k}h${-(w - 2 * k)}a${k} ${k} 0 0 1 ${-k} ${-k}v${-(h - 2 * k)}a${k} ${k} 0 0 1 ${k} ${-k}z`

export const ICONS = {
  // navigation / actions
  'arrow-right': ['M5 12h14', 'M13 6l6 6-6 6'], 'arrow-left': ['M19 12H5', 'M11 6l-6 6 6 6'], check: ['M5 12.5l4.5 4.5L19 7.5'],
  plus: ['M12 5v14M5 12h14'], lock: [r(5, 11, 14, 10), 'M8 11V8a4 4 0 0 1 8 0v3'],
  database: ['M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z', 'M4 6v6c0 1.7 3.6 3 8 3s8-1.3 8-3V6', 'M4 12v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6'],
  refresh: ['M20 11a8 8 0 0 0-14.5-3.5L4 9', 'M4 4v5h5', 'M4 13a8 8 0 0 0 14.5 3.5L20 15', 'M20 20v-5h-5'], user: [c(12, 8, 4), 'M4 21a8 8 0 0 1 16 0'],
  x: ['M6 6l12 12M18 6L6 18'], trash: ['M4 7h16', 'M9 7V4h6v3', 'M6 7l1 13h10l1-13', 'M10 11v6M14 11v6'], scissors: [c(6, 6, 3), c(6, 18, 3), 'M8.1 8.1L20 20', 'M8.1 15.9L20 4'],
  swap: ['M7 4L3 8l4 4', 'M3 8h14', 'M17 20l4-4-4-4', 'M21 16H7'], backspace: ['M9 5h11a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H9L3 12l6-7z', 'M12 9l5 6M17 9l-5 6'],
  'chevron-up': ['M6 15l6-6 6 6'], 'chevron-down': ['M6 9l6 6 6-6'], moon: ['M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5z'],
  sparkles: ['M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8L12 3z', 'M19 16l.8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8L19 16z'],
  printer: ['M7 8V3h10v5', r(3, 8, 18, 9), 'M7 14h10v7H7z'], monitor: [r(3, 4, 18, 12), 'M8 20h8', 'M12 16v4'],
  scale: ['M12 4v16', 'M7 20h10', 'M5 7h14', 'M5 7l-3 7a3 3 0 0 0 6 0L5 7z', 'M19 7l-3 7a3 3 0 0 0 6 0l-3-7z'], tag: ['M3 12V4h8l10 10-8 8L3 12z', 'M7.5 8h.01'],
  bookmark: ['M6 3h12v18l-6-4-6 4V3z'], 'rotate-ccw': ['M3 12a9 9 0 1 0 3-6.7L3 8', 'M3 3v5h5'], download: ['M12 4v11', 'M7 11l5 5 5-5', 'M4 20h16'],
  dot: [c(12, 12, 3)], grid: [r(3, 3, 7, 7, 1), r(14, 3, 7, 7, 1), r(3, 14, 7, 7, 1), r(14, 14, 7, 7, 1)], square: [r(4, 4, 16, 16, 2)], circle: [c(12, 12, 8)],
  // portal sites
  building: ['M4 21V5l8-2 8 2v16', 'M2 21h20', 'M9 8h2M13 8h2M9 12h2M13 12h2', 'M10 21v-4h4v4'],
  bag: ['M5 8h14l-1 12H6L5 8z', 'M9 8V6a3 3 0 0 1 6 0v2'], receipt: ['M6 3h12v18l-3-2-3 2-3-2-3 2V3z', 'M9 8h6M9 12h6'],
  coins: [c(12, 12, 9), 'M14.5 9.5c-.5-1-1.5-1.5-2.5-1.5-1.5 0-2.5.8-2.5 2s1 1.7 2.5 2 2.5.8 2.5 2-1 2-2.5 2c-1 0-2-.5-2.5-1.5', 'M12 6v2M12 16v2'],
  megaphone: ['M3 11v2a1 1 0 0 0 1 1h2l5 4V6L6 10H4a1 1 0 0 0-1 1z', 'M15 9a4 4 0 0 1 0 6', 'M18 6.5a8 8 0 0 1 0 11'], mail: [r(3, 5, 18, 14), 'M3 7l9 6 9-6'],
  box: ['M12 3l8 4.5v9L12 21l-8-4.5v-9L12 3z', 'M4 7.5l8 4.5 8-4.5', 'M12 12v9'],
  // catalog apps
  globe: [c(12, 12, 9), 'M3 12h18', 'M12 3a14 14 0 0 1 0 18', 'M12 3a14 14 0 0 0 0 18'], store: ['M3 9l1.5-5h15L21 9', 'M3 9v1a3 3 0 0 0 6 0 3 3 0 0 0 6 0 3 3 0 0 0 6 0V9', 'M5 13v8h14v-8', 'M10 21v-5h4v5'],
  newspaper: ['M5 4h12v16H7a2 2 0 0 1-2-2V4z', 'M17 8h2v10a2 2 0 0 1-2 2', 'M8 8h6M8 12h6M8 16h4'], message: ['M4 5h16v11H9l-5 4V5z'],
  graduation: ['M2 9l10-5 10 5-10 5L2 9z', 'M6 11v5c0 1.5 3 3 6 3s6-1.5 6-3v-5', 'M22 9v6'], ticket: ['M3 7h18v3a2 2 0 0 0 0 4v3H3v-3a2 2 0 0 0 0-4V7z', 'M14 7v10'],
  funnel: ['M3 4h18l-7 8v6l-4 2v-8L3 4z'], calculator: [r(4, 3, 16, 18), 'M8 7h8', 'M8 11h2M12 11h2M8 15h2M12 15h2'],
  utensils: ['M6 3v8a2 2 0 0 0 2 2v8', 'M10 3v8', 'M6 7h4', 'M17 3c-2 1-3 4-3 7h3v11'], repeat: ['M17 2l4 4-4 4', 'M3 11V9a3 3 0 0 1 3-3h15', 'M7 22l-4-4 4-4', 'M21 13v2a3 3 0 0 1-3 3H3'],
  key: [c(8, 15, 4), 'M11 12l9-9', 'M17 6l3 3', 'M14 9l2 2'], 'file-text': ['M6 3h8l4 4v14H6V3z', 'M14 3v4h4', 'M9 12h6M9 16h6'],
  book: ['M5 4h12a2 2 0 0 1 2 2v14H7a2 2 0 0 1-2-2V4z', 'M5 18a2 2 0 0 1 2-2h12', 'M9 8h6'], 'credit-card': [r(3, 5, 18, 14), 'M3 10h18', 'M7 15h4'],
  pen: ['M4 20l4-1 11-11-3-3L5 16l-1 4z', 'M14 6l3 3'], 'trending-up': ['M3 17l6-6 4 4 8-8', 'M15 7h6v6'], leaf: ['M5 19c0-9 5-14 15-15 0 10-5 15-14 15', 'M5 19c2-5 5-8 9-10'],
  columns: [r(3, 4, 18, 16), 'M9 4v16M15 4v16'], clock: [c(12, 12, 9), 'M12 7v5l3 2'], wrench: ['M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z'],
  headset: ['M4 14v-2a8 8 0 0 1 16 0v2', 'M4 14h3v5H5a1 1 0 0 1-1-1v-4z', 'M20 14h-3v5h2a1 1 0 0 0 1-1v-4z'], calendar: [r(3, 5, 18, 16), 'M3 10h18', 'M8 3v4M16 3v4'],
  'calendar-range': [r(3, 5, 18, 16), 'M3 10h18', 'M8 3v4M16 3v4', 'M7 14h5M10 17h7'], folder: ['M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6z'],
  'check-circle': [c(12, 12, 9), 'M8 12l3 3 5-6'], 'book-open': ['M12 6c-2-1.5-5-2-8-2v14c3 0 6 .5 8 2 2-1.5 5-2 8-2V4c-3 0-6 .5-8 2z', 'M12 6v14'],
  factory: ['M3 21V10l6 4v-4l6 4V6h3v15H3z', 'M7 17h2M12 17h2'], cart: ['M3 4h2l2 12h11l2-8H6', 'M10 20h.01M17 20h.01'], gear: [c(12, 12, 3), c(12, 12, 6.5), 'M12 2v3.5M12 18.5V22M2 12h3.5M18.5 12H22'],
  target: [c(12, 12, 9), c(12, 12, 5), c(12, 12, 1)], phone: [r(7, 2, 10, 20), 'M11 18h2'], clipboard: ['M8 4h8v3H8z', 'M6 5H5v16h14V5h-1', 'M9 12h6M9 16h6'],
  share: [c(6, 12, 2.5), c(18, 6, 2.5), c(18, 18, 2.5), 'M8.2 10.8l7.6-3.6M8.2 13.2l7.6 3.6'], users: ['M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2', c(9, 7, 4), 'M22 21v-2a4 4 0 0 0-3-3.87', 'M16 3.13a4 4 0 0 1 0 7.75'],
  'user-check': [c(9, 8, 4), 'M2 21a7 7 0 0 1 14 0', 'M16 11l2 2 4-4'], 'user-plus': [c(9, 8, 4), 'M2 21a7 7 0 0 1 14 0', 'M19 8v6M16 11h6'],
  sun: [c(12, 12, 4), 'M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9L17 7M7 17l-2.1 2.1'], star: ['M12 3l2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.2l-5.6 3 1.1-6.2L3 9.6l6.2-.9L12 3z'],
  car: ['M5 16H3v-4l2-5h14l2 5v4h-2', 'M3 12h18', c(7, 16, 2), c(17, 16, 2), 'M9 16h6'], banknote: [r(2, 6, 20, 12), c(12, 12, 3), 'M6 12h.01M18 12h.01'], layout: [r(3, 3, 18, 18), 'M3 9h18', 'M9 21V9'],
  // storefront
  shirt: ['M8 3L3 7l3 3 2-1v12h8V9l2 1 3-3-5-4a4 4 0 0 1-8 0z'], home: ['M3 11l9-8 9 8', 'M5 10v10h14V10', 'M10 20v-6h4v6'],
  gift: [r(3, 8, 18, 4, 1), 'M12 8v13', 'M5 12v9h14v-9', 'M12 8c-2 0-4-1-4-3s3-2 4 3z', 'M12 8c2 0 4-1 4-3s-3-2-4 3z'],
  // industry segments
  briefcase: [r(3, 7, 18, 13), 'M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2', 'M3 13h18'], palette: ['M12 3a9 9 0 1 0 0 18c1.5 0 2-1 1.5-2s-.5-2 1-2H17a4 4 0 0 0 4-4c0-5-4-10-9-10z', 'M7.5 11h.01M10 7.5h.01M15 7.5h.01'],
  heart: ['M12 20s-8-5-8-11a4.5 4.5 0 0 1 8-2.5A4.5 4.5 0 0 1 20 9c0 6-8 11-8 11z'], bed: ['M3 18V6', 'M3 14h18v4', 'M21 14v-2a3 3 0 0 0-3-3h-7v5', c(7, 11, 1.5)],
  hammer: ['M14 4l6 6-3 3-6-6 3-3z', 'M11 7L4 14l3 3 7-7'], pin: ['M12 21s-7-6-7-11a7 7 0 0 1 14 0c0 5-7 11-7 11z', c(12, 10, 2.5)],
} as const
export type IconName = keyof typeof ICONS

/** Catalog app (names from the trial catalog) -> icon. */
export const APP_ICON: Record<string, IconName> = {
  Website: 'globe', eCommerce: 'store', Blog: 'newspaper', Forum: 'message', eLearning: 'graduation', Events: 'ticket', CRM: 'funnel', Sales: 'coins', 'Point of Sale': 'calculator', Restaurant: 'utensils',
  Subscriptions: 'repeat', Rental: 'key', Invoicing: 'file-text', Accounting: 'book', Expenses: 'credit-card', Sign: 'pen', Equity: 'trending-up', ESG: 'leaf', Project: 'columns', Timesheets: 'clock',
  'Field Service': 'pin', Helpdesk: 'headset', Appointments: 'calendar', Planning: 'calendar-range', Documents: 'folder', Approvals: 'check-circle', Knowledge: 'book-open', Inventory: 'box',
  Manufacturing: 'factory', Purchase: 'cart', Maintenance: 'gear', Quality: 'target', Repair: 'wrench', 'Email Marketing': 'mail', 'SMS Marketing': 'phone', Survey: 'clipboard', 'Social Marketing': 'share',
  Employees: 'users', Attendances: 'user-check', Recruitment: 'user-plus', 'Time Off': 'sun', Appraisals: 'star', Fleet: 'car', Payroll: 'banknote', Studio: 'layout',
}
/** POS payment method kind -> icon. */
export const payIcon = (kind: string): IconName => (kind === 'cash' ? 'banknote' : kind === 'account' ? 'receipt' : 'credit-card')
export const iconForApp = (name: string): IconName => APP_ICON[name] ?? 'box'

/** Industry segment -> icon. */
export const SEGMENT_ICON: Record<string, IconName> = {
  'Business Services': 'briefcase', 'Culture & Arts': 'palette', 'Education & Training': 'graduation', 'Events, Community & Nonprofits': 'users', 'Food & Beverage': 'utensils', 'Health, Wellness & Personal Care': 'heart',
  'Hospitality, Tourism & Leisure': 'bed', 'Manufacturing & Supply Chain': 'factory', 'Real Estate, Construction & Maintenance': 'hammer', 'Retail & eCommerce': 'store', 'Trades & Home Services': 'wrench',
}
