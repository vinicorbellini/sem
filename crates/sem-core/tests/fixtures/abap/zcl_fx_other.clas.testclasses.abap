CLASS ltc_other DEFINITION DEFERRED.
CLASS zcl_fx_other DEFINITION LOCAL FRIENDS ltc_other.

CLASS ltc_other DEFINITION FINAL FOR TESTING
  DURATION SHORT RISK LEVEL HARMLESS.

  PRIVATE SECTION.
    METHODS label_has_tag FOR TESTING.
ENDCLASS.


CLASS ltc_other IMPLEMENTATION.

  METHOD label_has_tag.
    DATA(lo_cut) = NEW zcl_fx_other( ).
    cl_abap_unit_assert=>assert_char_cp(
      act = lo_cut->label( zcl_fx_order=>create( 1 ) )
      exp = 'other*' ).
  ENDMETHOD.

ENDCLASS.
