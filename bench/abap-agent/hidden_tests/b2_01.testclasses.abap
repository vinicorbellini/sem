
CLASS ltcl_hidden_b2 DEFINITION FINAL FOR TESTING
  DURATION SHORT
  RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    METHODS:
      condense_true FOR TESTING,
      condense_false FOR TESTING.

ENDCLASS.

CLASS ltcl_hidden_b2 IMPLEMENTATION.

  METHOD condense_true.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_git_branch_utils=>complete_heads_branch_name(
              iv_branch_name = ` feature `
              iv_condense    = abap_true )
      exp = 'refs/heads/feature' ).

  ENDMETHOD.

  METHOD condense_false.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_git_branch_utils=>complete_heads_branch_name(
              iv_branch_name = 'refs/heads/feature'
              iv_condense    = abap_false )
      exp = 'refs/heads/feature' ).

  ENDMETHOD.

ENDCLASS.
