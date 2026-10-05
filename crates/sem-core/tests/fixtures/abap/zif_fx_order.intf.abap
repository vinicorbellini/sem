INTERFACE zif_fx_order PUBLIC.

  TYPES ty_amount TYPE p LENGTH 9 DECIMALS 2.

  CONSTANTS c_status_open TYPE c LENGTH 1 VALUE 'O'.

  METHODS add_item
    IMPORTING iv_name TYPE string
              iv_amount TYPE zfx_amount.

  METHODS get_total
    RETURNING VALUE(rv_total) TYPE zif_fx_order=>ty_amount.

ENDINTERFACE.
