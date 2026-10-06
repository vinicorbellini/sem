CLASS zcl_fx_chain DEFINITION PUBLIC CREATE PUBLIC.
  PUBLIC SECTION.
    TYPES: ty_id   TYPE i,
           ty_name TYPE string.
    DATA: mv_id   TYPE ty_id,
          mv_name TYPE ty_name.
    CLASS-DATA: gv_count TYPE i,
                BEGIN OF gs_last,
                  id   TYPE ty_id,
                  name TYPE ty_name,
                END OF gs_last,
                gv_total TYPE i.
    METHODS describe RETURNING VALUE(rv_text) TYPE string.
  PRIVATE SECTION.
    DATA mv_note TYPE string.
ENDCLASS.

CLASS zcl_fx_chain IMPLEMENTATION.
  METHOD describe.
    DATA: lv_a TYPE i,
          lv_b TYPE string.
    rv_text = |{ mv_id } { mv_name } { mv_note }|.
  ENDMETHOD.
ENDCLASS.
