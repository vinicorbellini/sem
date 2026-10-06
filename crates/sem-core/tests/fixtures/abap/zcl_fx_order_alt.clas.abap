* A second zif_fx_order, through zif_fx_audit, with an alias (story 2.3)
CLASS zcl_fx_order_alt DEFINITION PUBLIC CREATE PUBLIC.

  PUBLIC SECTION.
    INTERFACES zif_fx_audit.
    ALIASES total FOR zif_fx_order~get_total.

  PRIVATE SECTION.
    DATA mv_total TYPE zif_fx_order=>ty_amount.
ENDCLASS.


CLASS zcl_fx_order_alt IMPLEMENTATION.

  METHOD zif_fx_order~add_item.
    ADD iv_amount TO mv_total.
  ENDMETHOD.

  METHOD zif_fx_order~get_total.
    rv_total = mv_total.
  ENDMETHOD.

  METHOD zif_fx_audit~audit.
    total( ).
  ENDMETHOD.

  METHOD zif_fx_audit~log.
    rv_log = me->total( ).
  ENDMETHOD.

ENDCLASS.
