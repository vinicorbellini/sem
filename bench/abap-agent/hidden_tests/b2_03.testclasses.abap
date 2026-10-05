
CLASS ltcl_hidden_b2 DEFINITION FINAL FOR TESTING
  DURATION SHORT
  RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    METHODS:
      newline_true FOR TESTING RAISING zcx_abapgit_exception,
      newline_false FOR TESTING RAISING zcx_abapgit_exception.

ENDCLASS.

CLASS ltcl_hidden_b2 IMPLEMENTATION.

  METHOD newline_true.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_git_utils=>pkt_string(
              iv_string  = 'abc'
              iv_newline = abap_true )
      exp = |0008abc\n| ).

  ENDMETHOD.

  METHOD newline_false.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_git_utils=>pkt_string(
              iv_string  = 'abc'
              iv_newline = abap_false )
      exp = '0007abc' ).

  ENDMETHOD.

ENDCLASS.
