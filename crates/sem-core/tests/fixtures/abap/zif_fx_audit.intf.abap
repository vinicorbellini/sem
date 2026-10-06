* An interface that includes another (story 2.3)
INTERFACE zif_fx_audit PUBLIC.

  INTERFACES zif_fx_order.

  METHODS: audit IMPORTING iv_note TYPE string,
    log RETURNING VALUE(rv_log) TYPE string.

ENDINTERFACE.
