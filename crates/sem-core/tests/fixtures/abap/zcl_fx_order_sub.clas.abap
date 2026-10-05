CLASS zcl_fx_order_sub DEFINITION PUBLIC
  INHERITING FROM zcl_fx_order CREATE PUBLIC.

  PUBLIC SECTION.
    METHODS describe REDEFINITION.
ENDCLASS.


CLASS zcl_fx_order_sub IMPLEMENTATION.

  METHOD describe.
    rv_text = |sub: { super->describe( ) }|.
  ENDMETHOD.

ENDCLASS.
