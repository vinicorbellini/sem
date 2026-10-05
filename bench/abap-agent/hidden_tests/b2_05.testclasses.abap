
CLASS ltcl_hidden_b2 DEFINITION FINAL FOR TESTING
  DURATION SHORT
  RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    METHODS:
      upper_and_lower FOR TESTING RAISING zcx_abapgit_exception.

ENDCLASS.

CLASS ltcl_hidden_b2 IMPLEMENTATION.

  METHOD upper_and_lower.

    DATA: lv_data  TYPE xstring,
          lv_lower TYPE zif_abapgit_git_definitions=>ty_sha1,
          lv_upper TYPE zif_abapgit_git_definitions=>ty_sha1,
          lv_exp   TYPE zif_abapgit_git_definitions=>ty_sha1.

    lv_data = '616263'.

    lv_lower = zcl_abapgit_hash=>sha1_raw(
      iv_data       = lv_data
      iv_upper_case = abap_false ).
    lv_upper = zcl_abapgit_hash=>sha1_raw(
      iv_data       = lv_data
      iv_upper_case = abap_true ).

    lv_exp = to_lower( lv_lower ).
    cl_abap_unit_assert=>assert_equals(
      act = lv_lower
      exp = lv_exp ).

    lv_exp = to_upper( lv_lower ).
    cl_abap_unit_assert=>assert_equals(
      act = lv_upper
      exp = lv_exp ).

    cl_abap_unit_assert=>assert_differs(
      act = lv_upper
      exp = lv_lower ).

  ENDMETHOD.

ENDCLASS.
