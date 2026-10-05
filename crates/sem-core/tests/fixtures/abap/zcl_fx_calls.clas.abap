* One method per static call form, each in both spellings (story 2.1)
CLASS zcl_fx_calls DEFINITION PUBLIC
  INHERITING FROM zcl_fx_order CREATE PUBLIC.

  PUBLIC SECTION.
    METHODS describe REDEFINITION.
    METHODS me_call.
    METHODS static_call.
    METHODS interface_call.
    METHODS call_function.
    METHODS new_object.
    METHODS unknown_receiver.
  PRIVATE SECTION.
    DATA mo_other TYPE REF TO zcl_fx_other.
ENDCLASS.


CLASS zcl_fx_calls IMPLEMENTATION.

  METHOD describe.
    rv_text = super->describe( ).
    CALL METHOD SUPER->DESCRIBE RECEIVING rv_text = rv_text.
  ENDMETHOD.

  METHOD me_call.
    me->static_call( ).
    STATIC_CALL( ).
    CALL METHOD me->static_call.
  ENDMETHOD.

  METHOD static_call.
    zcl_fx_order=>create( 1 ).
    ZCL_FX_ORDER=>CREATE( iv_id = 2 ).
    CALL METHOD zcl_fx_order=>create EXPORTING iv_id = 3.
  ENDMETHOD.

  METHOD interface_call.
    DATA(lv_total) = zif_fx_order~get_total( ).
    lv_total = me->ZIF_FX_ORDER~GET_TOTAL( ).
  ENDMETHOD.

  METHOD call_function.
    CALL FUNCTION 'ZFX_FM' EXPORTING iv_id = 1.
    CALL FUNCTION 'zfx_fm'.
  ENDMETHOD.

  METHOD new_object.
    DATA(lo_order) = NEW zcl_fx_order( 1 ).
    mo_other = NEW ZCL_FX_OTHER( ).
  ENDMETHOD.

  METHOD unknown_receiver.
    DATA(lo_other) = NEW zcl_fx_other( ).
    lo_other->total( me ).
    CALL METHOD lo_other->total EXPORTING io_order = me.
    mo_other->total( me ).
  ENDMETHOD.

ENDCLASS.
