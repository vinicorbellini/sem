* Global class of the fixture repository (classic comment, column 1)
CLASS zcl_fx_order DEFINITION PUBLIC CREATE PUBLIC.

  PUBLIC SECTION.
    INTERFACES zif_fx_order.
    METHODS constructor IMPORTING iv_id TYPE i.
    METHODS describe RETURNING VALUE(rv_text) TYPE string.
    CLASS-METHODS create IMPORTING iv_id TYPE i
      RETURNING VALUE(ro_order) TYPE REF TO zcl_fx_order.

  PRIVATE SECTION.
    DATA mv_id TYPE i.
    DATA mt_names TYPE string_table.
    DATA mv_total TYPE zif_fx_order=>ty_amount.
ENDCLASS.


CLASS zcl_fx_order IMPLEMENTATION.

  METHOD constructor.
    mv_id = iv_id.   " trailing comment: call describe( ) later
  ENDMETHOD.

  METHOD create.
    ro_order = NEW #( iv_id = iv_id ).
  ENDMETHOD.

  METHOD describe.
    DATA(lo_helper) = NEW lcl_helper( ).
    rv_text = |order { mv_id } total { zif_fx_order~get_total( ) } { lo_helper->tag( ) }|.
  ENDMETHOD.

  METHOD zif_fx_order~add_item.
    APPEND iv_name TO mt_names.
    ADD iv_amount TO mv_total.
  ENDMETHOD.

  METHOD zif_fx_order~get_total.
    DATA lv_log TYPE string.
    lv_log = 'calls get_total( ) and describe( ) in a literal'.
    rv_total = mv_total.
  ENDMETHOD.

ENDCLASS.
