* Calls through an interface, a base class and an alias (story 2.3)
CLASS zcl_fx_dispatch DEFINITION PUBLIC CREATE PUBLIC.

  PUBLIC SECTION.
    METHODS: through_interface IMPORTING lif TYPE REF TO zif_fx_order,
      through_base IMPORTING lo_base TYPE REF TO zcl_fx_order,
      through_alias IMPORTING lo_alt TYPE REF TO zcl_fx_order_alt,
      through_outer IMPORTING lo_audit TYPE REF TO zif_fx_audit.
ENDCLASS.


CLASS zcl_fx_dispatch IMPLEMENTATION.

  METHOD through_interface.
    lif->get_total( ).
  ENDMETHOD.

  METHOD through_base.
    lo_base->describe( ).
  ENDMETHOD.

  METHOD through_alias.
    lo_alt->total( ).
  ENDMETHOD.

  METHOD through_outer.
    lo_audit->zif_fx_order~get_total( ).
    lo_audit->audit( `checked` ).
  ENDMETHOD.

ENDCLASS.
