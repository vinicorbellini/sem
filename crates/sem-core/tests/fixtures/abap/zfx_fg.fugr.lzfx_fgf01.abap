*----------------------------------------------------------------------*
***INCLUDE LZFX_FGF01.
*----------------------------------------------------------------------*

FORM calc_extra USING iv_id TYPE i CHANGING cv_total TYPE i.
  cv_total = cv_total + gv_extra + iv_id.   " classic arithmetic
ENDFORM.
