
CLASS ltcl_hidden_b2 DEFINITION FINAL FOR TESTING
  DURATION SHORT
  RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    METHODS:
      format_xml FOR TESTING,
      format_default FOR TESTING.

ENDCLASS.

CLASS ltcl_hidden_b2 IMPLEMENTATION.

  METHOD format_xml.

    DATA ls_config TYPE zif_abapgit_data_config=>ty_config.

    ls_config-name = '/NSPC/T200'.
    ls_config-type = 'TABU'.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_data_utils=>build_data_filename(
              is_config = ls_config
              iv_format = 'xml' )
      exp = '#nspc#t200.tabu.xml' ).

  ENDMETHOD.

  METHOD format_default.

    DATA ls_config TYPE zif_abapgit_data_config=>ty_config.

    ls_config-name = 'T100'.
    ls_config-type = 'TABU'.

    cl_abap_unit_assert=>assert_equals(
      act = zcl_abapgit_data_utils=>build_data_filename(
              is_config = ls_config
              iv_format = zif_abapgit_data_config=>c_default_format )
      exp = 't100.tabu.json' ).

  ENDMETHOD.

ENDCLASS.
