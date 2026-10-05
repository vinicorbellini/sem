* Uses zcl_fx_order and zcl_fx_other from another object (story 2.0)
CLASS zcl_fx_user DEFINITION PUBLIC CREATE PUBLIC.

  PUBLIC SECTION.
    METHODS run RETURNING VALUE(rv_text) TYPE string.
    METHODS describe RETURNING VALUE(rv_text) TYPE string.
ENDCLASS.


CLASS zcl_fx_user IMPLEMENTATION.

  METHOD run.
    DATA(lo_order) = ZCL_FX_ORDER=>CREATE( 1 ).
    DATA(lo_other) = NEW zcl_fx_other( ).
    rv_text = lo_other->label( lo_order ).
  ENDMETHOD.

  METHOD describe.
    rv_text = 'user'.   " zcl_fx_order defines a describe too
  ENDMETHOD.

ENDCLASS.
