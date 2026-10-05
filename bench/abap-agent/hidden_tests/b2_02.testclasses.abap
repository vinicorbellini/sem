
CLASS ltcl_hidden_b2 DEFINITION FINAL FOR TESTING
  DURATION SHORT
  RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    METHODS:
      unescape_true FOR TESTING,
      unescape_false FOR TESTING.

ENDCLASS.

CLASS ltcl_hidden_b2 IMPLEMENTATION.

  METHOD unescape_true.

    DATA: lv_path     TYPE string,
          lv_filename TYPE string.

    zcl_abapgit_path=>split_file_location(
      EXPORTING
        iv_fullpath = '/src/zcl_a%23b.clas.abap'
        iv_unescape = abap_true
      IMPORTING
        ev_path     = lv_path
        ev_filename = lv_filename ).

    cl_abap_unit_assert=>assert_equals(
      act = lv_path
      exp = '/src/' ).
    cl_abap_unit_assert=>assert_equals(
      act = lv_filename
      exp = 'zcl_a#b.clas.abap' ).

  ENDMETHOD.

  METHOD unescape_false.

    DATA: lv_path     TYPE string,
          lv_filename TYPE string.

    zcl_abapgit_path=>split_file_location(
      EXPORTING
        iv_fullpath = '/src/zcl_a%23b.clas.abap'
        iv_unescape = abap_false
      IMPORTING
        ev_path     = lv_path
        ev_filename = lv_filename ).

    cl_abap_unit_assert=>assert_equals(
      act = lv_path
      exp = '/src/' ).
    cl_abap_unit_assert=>assert_equals(
      act = lv_filename
      exp = 'zcl_a%23b.clas.abap' ).

  ENDMETHOD.

ENDCLASS.
