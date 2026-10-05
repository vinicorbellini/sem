* Fixture report: reaches a form of an include it names, and none it does not
REPORT zfx_report2.

INCLUDE zfx_report_f01.

FORM run_report USING iv_total TYPE i CHANGING cv_text TYPE string.
  PERFORM format_total USING iv_total CHANGING cv_text.
ENDFORM.
