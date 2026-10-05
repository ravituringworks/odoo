import { groupsVisible, resolveTyped } from './m2o'
const eq = (a: unknown, b: unknown, m: string) => { if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL', m, a, b); process.exit(1) } }
eq(resolveTyped('muah', []), null, 'nothing found'); eq(resolveTyped('  ', [[1, 'x']]), null, 'blank')
eq(resolveTyped('acme', [[1, 'Acme Inc'], [2, 'ACME']]), [2, 'ACME'], 'exact match wins'); eq(resolveTyped('ac', [[1, 'Acme Inc']]), [1, 'Acme Inc'], 'single suggestion')
eq(resolveTyped('ac', [[1, 'Acme Inc'], [2, 'Acorn']]), null, 'ambiguous reverts')
eq(groupsVisible(undefined), true, 'no groups'); eq(groupsVisible('base.group_no_one'), false, 'debug only'); eq(groupsVisible('website.group_multi_website'), false, 'multi-website')
eq(groupsVisible('base.group_user,base.group_no_one'), true, 'any visible group shows it'); eq(groupsVisible('!base.group_no_one'), true, 'negation')
console.log('m2o ok')
