* Fixture report: one statement per computed call, a static call, and noise.
REPORT zfx_dynamic.

DATA: lv_fm   TYPE rs38l_fnam VALUE 'ZFX_FM',
      lv_meth TYPE string VALUE 'DESCRIBE',
      lv_form TYPE string VALUE 'SHOW_ORDER',
      lv_prog TYPE sy-repid VALUE 'ZFX_REPORT',
      lv_cls  TYPE seoclsname VALUE 'ZCL_FX_ORDER',
      lv_text TYPE string,
      lo_any  TYPE REF TO object.

* zfx_fm and describe are only named in this comment
START-OF-SELECTION.
  CALL FUNCTION lv_fm.
  CALL METHOD zcl_fx_order=>(lv_meth).
  CALL METHOD lo_any->(lv_meth).
  PERFORM (lv_form) IN PROGRAM zfx_report.
  PERFORM show_order IN PROGRAM (lv_prog).
  CREATE OBJECT lo_any TYPE (lv_cls).
  DATA(lo_order) = NEW zcl_fx_order( 1 ).
  lv_text = lo_any->describe( ).
  lv_text = zcl_fx_order=>create( 2 )->describe( ).
  lv_text = |zfx_fm { lv_text } describe|.
  WRITE: / 'zfx_fm describe show_order'.
