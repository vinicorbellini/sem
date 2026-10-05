*&---------------------------------------------------------------------*
*& Include zfx_report_f01
*&---------------------------------------------------------------------*

FORM format_total USING iv_total TYPE i CHANGING cv_text TYPE string.
  cv_text = |total: { iv_total }|.   " trailing comment
ENDFORM.
