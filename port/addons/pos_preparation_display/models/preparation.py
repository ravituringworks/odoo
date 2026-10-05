# -*- coding: utf-8 -*-
from odoo import fields, models


class PosPrepDisplay(models.Model):
    _name = 'pos_preparation_display.display'
    _description = 'Preparation Display'
    _order = 'name'

    name = fields.Char(required=True)
    pos_config_ids = fields.Many2many('pos.config', string='Point of Sales')
    category_ids = fields.Many2many('pos.category', string='Product Categories',
                                    help='Only these categories show on this display. Empty = everything.')
    ticket_count = fields.Integer(compute='_compute_ticket_count')

    def _compute_ticket_count(self):
        for d in self:
            d.ticket_count = self.env['pos_preparation_display.ticket'].search_count([('state', '!=', 'done')])


class PosPrepTicket(models.Model):
    _name = 'pos_preparation_display.ticket'
    _description = 'Preparation Ticket'
    _order = 'id'

    name = fields.Char(required=True)
    config_id = fields.Many2one('pos.config', required=True, ondelete='cascade')
    order_id = fields.Many2one('pos.order', ondelete='cascade')
    table_id = fields.Many2one('restaurant.table')
    takeaway = fields.Boolean()
    note = fields.Char()
    state = fields.Selection([('new', 'To prepare'), ('cooking', 'Preparing'), ('done', 'Ready')], default='new', required=True)
    line_ids = fields.One2many('pos_preparation_display.line', 'ticket_id', string='Lines')


class PosPrepLine(models.Model):
    _name = 'pos_preparation_display.line'
    _description = 'Preparation Line'

    ticket_id = fields.Many2one('pos_preparation_display.ticket', required=True, ondelete='cascade')
    product_id = fields.Many2one('product.product')
    name = fields.Char(required=True)
    qty = fields.Float(default=1.0)
    note = fields.Char()
    cancelled = fields.Boolean(help='The waiter removed this item after sending it.')
    category_ids = fields.Many2many('pos.category', string='Categories')
    state = fields.Selection([('new', 'To prepare'), ('cooking', 'Preparing'), ('done', 'Ready')], default='new', required=True)
