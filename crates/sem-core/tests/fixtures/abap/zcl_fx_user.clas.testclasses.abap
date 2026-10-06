* Tests that reach zcl_fx_order through other classes (story 2.6)
CLASS ltc_user DEFINITION FINAL FOR TESTING
  DURATION SHORT RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    DATA mo_cut TYPE REF TO zcl_fx_user.
    METHODS setup.
    METHODS run_labels_order FOR TESTING.
    METHODS describe_is_user FOR TESTING.
    METHODS total_through_interface FOR TESTING.
    METHODS describe_through_base FOR TESTING.
ENDCLASS.


CLASS ltc_user IMPLEMENTATION.

  METHOD setup.
    mo_cut = NEW #( ).
  ENDMETHOD.

  METHOD run_labels_order.
    cl_abap_unit_assert=>assert_char_cp( act = mo_cut->run( ) exp = '*order 1*' ).
  ENDMETHOD.

  METHOD describe_is_user.
    cl_abap_unit_assert=>assert_equals( act = mo_cut->describe( ) exp = 'user' ).
  ENDMETHOD.

  METHOD total_through_interface.
    DATA lif TYPE REF TO zif_fx_order.
    lif = NEW zcl_fx_order_alt( ).
    cl_abap_unit_assert=>assert_equals( act = lif->get_total( ) exp = 0 ).
  ENDMETHOD.

  METHOD describe_through_base.
    DATA lo_base TYPE REF TO zcl_fx_order.
    lo_base = NEW zcl_fx_order_sub( 3 ).
    cl_abap_unit_assert=>assert_char_cp( act = lo_base->describe( ) exp = 'sub*' ).
  ENDMETHOD.

ENDCLASS.
