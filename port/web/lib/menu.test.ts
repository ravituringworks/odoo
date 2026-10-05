import { adminRoot, leafHref, visibleApps } from './menu'
import type { Menu } from './hooks'
const m = (id: string, name: string, model: string | null = null, children: Menu[] = []): Menu => ({ id, name, sequence: 1, model, view_mode: 'list,form', children })
const eq = (a: unknown, b: unknown, msg: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', msg, JSON.stringify(a), JSON.stringify(b)); process.exit(1) } }
const menus = [m('sale.root', 'Sales'), m('base.menu_management', 'Apps'), m('base.menu_administration', 'Settings')]
eq(visibleApps(menus).map((x) => x.name), ['Sales'], 'odoo Settings/Apps roots hidden'); eq(adminRoot(menus)?.name, 'Settings', 'admin root found')
eq(leafHref(m('x', 'General Settings', 'res.config.settings')), '/settings/', 'config settings -> unified settings')
eq(leafHref(m('y', 'Settings', 'res.config.settings')), '/settings/', 'per-app settings -> unified settings'); eq(leafHref(m('z', 'Main Apps', 'ir.module.module')), '/apps/', 'module list -> apps page')
eq(leafHref(m('o', 'Orders', 'sale.order')).startsWith('/list/?model=sale.order'), true, 'normal leaf'); console.log('menu: ok')
