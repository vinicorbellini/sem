*"* use this source file for the definition and implementation of
*"* local helper classes, interface definitions and type
*"* declarations

CLASS lcl_helper DEFINITION FINAL.
  PUBLIC SECTION.
    METHODS tag RETURNING VALUE(rv_tag) TYPE string.
ENDCLASS.


CLASS lcl_helper IMPLEMENTATION.

  METHOD tag.
    rv_tag = 'other'.   " same name as zcl_fx_order's local class
  ENDMETHOD.

ENDCLASS.
