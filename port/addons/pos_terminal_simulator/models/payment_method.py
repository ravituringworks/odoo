# -*- coding: utf-8 -*-
from odoo import fields, models


class PosPaymentMethod(models.Model):
    _inherit = 'pos.payment.method'

    use_payment_terminal = fields.Selection(selection_add=[('simulator', 'Simulator (testing)')])
