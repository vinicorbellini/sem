
CLASS ltcl_hidden_b2 DEFINITION FINAL FOR TESTING
  DURATION SHORT
  RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    METHODS:
      trailing_true FOR TESTING,
      trailing_false FOR TESTING.

ENDCLASS.

CLASS ltcl_hidden_b2 IMPLEMENTATION.

  METHOD trailing_true.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_string_buffer=>new( )->add( 'a' )->add( 'b' )->join_w_newline_and_flush(
              iv_trailing_newline = abap_true )
      exp = |a\nb\n| ).

  ENDMETHOD.

  METHOD trailing_false.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_string_buffer=>new( )->add( 'a' )->add( 'b' )->join_w_newline_and_flush(
              iv_trailing_newline = abap_false )
      exp = |a\nb| ).

  ENDMETHOD.

ENDCLASS.
