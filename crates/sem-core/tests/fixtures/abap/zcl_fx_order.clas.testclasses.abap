CLASS ltc_order DEFINITION FINAL FOR TESTING
  DURATION SHORT RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    DATA mo_cut TYPE REF TO zcl_fx_order.
    METHODS setup.
    METHODS total_starts_at_zero FOR TESTING.
    METHODS describe_mentions_id FOR TESTING.
ENDCLASS.


CLASS ltc_order IMPLEMENTATION.

  METHOD setup.
    mo_cut = zcl_fx_order=>create( 7 ).
  ENDMETHOD.

  METHOD total_starts_at_zero.
    cl_abap_unit_assert=>assert_equals( act = mo_cut->zif_fx_order~get_total( ) exp = 0 ).
  ENDMETHOD.

  METHOD describe_mentions_id.
    cl_abap_unit_assert=>assert_char_cp( act = mo_cut->describe( ) exp = '*order 7*' ).
  ENDMETHOD.

ENDCLASS.
