from odoo import api, fields, models
class LibraryBook(models.Model):
    _name = 'library.book'
    _description = 'Library Book'
    name = fields.Char(required=True)
    isbn = fields.Char()
    author_id = fields.Many2one('res.partner', string='Author')
    state = fields.Selection([('available', 'Available'), ('lent', 'Lent')], default='available')
    pages = fields.Integer()
    def action_lend(self):
        self.write({'state': 'lent'})
