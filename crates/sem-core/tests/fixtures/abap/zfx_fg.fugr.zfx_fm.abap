FUNCTION zfx_fm.
*"----------------------------------------------------------------------
*"*"Local Interface:
*"  IMPORTING
*"     VALUE(IV_ID) TYPE  I
*"  EXPORTING
*"     VALUE(EV_TOTAL) TYPE  I
*"----------------------------------------------------------------------

  DATA(lo_order) = zcl_fx_order=>create( iv_id ).
  ev_total = lo_order->zif_fx_order~get_total( ).
  PERFORM calc_extra USING iv_id CHANGING ev_total.

ENDFUNCTION.
