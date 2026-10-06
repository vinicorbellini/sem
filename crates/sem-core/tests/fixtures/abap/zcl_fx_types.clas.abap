* Receivers bound by their declared types, one method per form (story 2.2)
CLASS zcl_fx_types DEFINITION PUBLIC CREATE PUBLIC.
  PUBLIC SECTION.
    TYPES ty_order TYPE REF TO zcl_fx_order.
    METHODS: by_param IMPORTING io_order TYPE REF TO zcl_fx_order
                        CHANGING co_order TYPE ty_order,
      by_return RETURNING VALUE(ro_order) TYPE REF TO zcl_fx_order,
      chain,
      by_cast IMPORTING io_any TYPE REF TO object,
      by_create,
      by_new_hash,
      untyped IMPORTING io_obj TYPE REF TO object io_data TYPE REF TO data,
      outside,
      no_local_class.
  PRIVATE SECTION.
    DATA mo_order TYPE REF TO zcl_fx_order.
ENDCLASS.
CLASS zcl_fx_types IMPLEMENTATION.
  METHOD by_param.
    io_order->describe( ).
    co_order->describe( ).
  ENDMETHOD.
  METHOD by_return.
    ro_order = NEW #( 1 ).
  ENDMETHOD.
  METHOD chain.
    zcl_fx_order=>create( 1 )->describe( ).
    by_return( )->zif_fx_order~get_total( ).
  ENDMETHOD.
  METHOD by_cast.
    CAST zif_fx_order( io_any )->get_total( ).
    CAST zcl_fx_order( io_any )->describe( ).
  ENDMETHOD.
  METHOD by_create.
    DATA: lo_order TYPE REF TO zcl_fx_order, lo_base TYPE REF TO zcl_fx_order.
    CREATE OBJECT lo_order EXPORTING iv_id = 1.
    lo_order->describe( ).
    CREATE OBJECT lo_base TYPE zcl_fx_order_sub EXPORTING iv_id = 2.
    lo_base->describe( ).
  ENDMETHOD.
  METHOD by_new_hash.
    mo_order = NEW #( 2 ).
    mo_order->describe( ).
  ENDMETHOD.
  METHOD untyped.
    io_obj->describe( ).
    io_data->describe( ).
    lo_nowhere->describe( ).
  ENDMETHOD.
  METHOD outside.
    DATA lo_descr TYPE REF TO cl_abap_typedescr.
    lo_descr->get_relative_name( ).
    cl_abap_unit_assert=>fail( ).
  ENDMETHOD.
  METHOD no_local_class.
    DATA lo_helper TYPE REF TO lcl_helper.
    lo_helper->tag( ).
  ENDMETHOD.
ENDCLASS.
