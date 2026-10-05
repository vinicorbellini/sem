* Fixture report: its own show_order, and calls that must not reach other programs
REPORT zfx_other.

FORM show_order USING iv_id TYPE i.
  WRITE: / iv_id.
ENDFORM.

FORM run_own USING iv_id TYPE i.
  PERFORM show_order USING iv_id.
ENDFORM.

FORM run_remote USING iv_id TYPE i.
  PERFORM show_order IN PROGRAM zfx_report USING iv_id.
ENDFORM.

FORM run_dynamic USING iv_name TYPE string.
  PERFORM show_order IN PROGRAM (iv_name).
ENDFORM.

FORM run_leak CHANGING cv_text TYPE string.
  PERFORM format_total USING 1 CHANGING cv_text.
ENDFORM.
