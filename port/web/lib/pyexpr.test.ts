import assert from 'node:assert/strict'
import { actionRef, pyToJson } from './pyexpr'

assert.deepEqual(pyToJson("[('campaign_id', '=', active_id)]", { active_id: 5 }), [['campaign_id', '=', 5]])
assert.deepEqual(pyToJson("[('state', 'in', ('a','b')), ('active', '=', True)]"), [['state', 'in', ['a', 'b']], ['active', '=', true]])
assert.deepEqual(pyToJson("{'default_campaign_id': active_id, 'search_default_x': 1, 'q': \"it's\"}", { active_id: 7 }), { default_campaign_id: 7, search_default_x: 1, q: "it's" })
assert.deepEqual(pyToJson("[('user_id','=',uid)]", { uid: 2 }), [['user_id', '=', 2]])
assert.equal(pyToJson("[('a', '=', some_call())]"), null); assert.equal(pyToJson(''), null); assert.deepEqual(pyToJson([1]), [1])
assert.equal(actionRef('%(mass_mailing.action_view_mass_mailings_from_campaign)d'), 'mass_mailing.action_view_mass_mailings_from_campaign'); assert.equal(actionRef('action_x'), null)
console.log('pyexpr ok')
