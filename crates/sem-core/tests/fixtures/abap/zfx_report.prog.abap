* Fixture report: FORM, PERFORM, macro, function call, include
REPORT zfx_report.

INCLUDE zfx_report_f01.

DEFINE _log.
  WRITE: / &1.
END-OF-DEFINITION.

PARAMETERS p_id TYPE i DEFAULT 1.

START-OF-SELECTION.
  DATA lv_total TYPE i.
  PERFORM show_order USING p_id CHANGING lv_total.
  CALL FUNCTION 'ZFX_FM'
    EXPORTING iv_id = p_id
    IMPORTING ev_total = lv_total.
  _log 'done'.   " macro call, mentions show_order
  WRITE: / 'show_order is only a string here'.

FORM show_order USING iv_id TYPE i CHANGING cv_total TYPE i.
  DATA(lo_order) = NEW zcl_fx_order( iv_id ).
  cv_total = lines( VALUE string_table( ( lo_order->describe( ) ) ) ).
ENDFORM.
