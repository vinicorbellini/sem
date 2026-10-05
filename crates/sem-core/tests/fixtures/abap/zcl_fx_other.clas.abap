* A second global class with its own local lcl_helper (story 2.0)
CLASS zcl_fx_other DEFINITION PUBLIC CREATE PUBLIC.

  PUBLIC SECTION.
    METHODS label IMPORTING io_order TYPE REF TO zcl_fx_order
      RETURNING VALUE(rv_text) TYPE string.
    METHODS total IMPORTING io_order TYPE REF TO zcl_fx_order
      RETURNING VALUE(rv_total) TYPE zif_fx_order=>ty_amount.
ENDCLASS.


CLASS zcl_fx_other IMPLEMENTATION.

  METHOD label.
    DATA(lo_helper) = NEW lcl_helper( ).
    rv_text = |{ lo_helper->tag( ) } { io_order->describe( ) }|.
  ENDMETHOD.

  METHOD total.
    rv_total = io_order->zif_fx_order~get_total( ).
  ENDMETHOD.

ENDCLASS.
