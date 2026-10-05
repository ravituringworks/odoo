# -*- coding: utf-8 -*-
{
    'name': 'POS Preparation Display',
    'version': '1.0',
    'category': 'Sales/Point of Sale',
    'summary': 'Kitchen / bar display: tickets sent from the POS, advanced by the cooks',
    'depends': ['point_of_sale', 'pos_restaurant'],
    'data': ['security/ir.model.access.csv', 'views/preparation_views.xml'],
    'installable': True,
    'license': 'LGPL-3',
}
