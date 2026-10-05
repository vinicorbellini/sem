# Tier 2 candidates

Gaps found after Tier 1 that no Tier 1 story owns. Each line names where it was
found. Promoting one to a story is a decision, not a default.

- **T2-A Recover METHOD blocks from the token stream** (Gate 1 census,
  `census-gate1.md`: #1928, #3185, #3891, #4432, #5072, #5711, #6669, #7644).
  The grammar's error recovery drops the rest of a `CLASS ... IMPLEMENTATION`
  or stretches a `method_implementation` past its `ENDMETHOD` to the end of
  the class. Only 68% of abapGit's 7577 `METHOD` blocks become a method
  entity, and 94 method entities span another method. A fallback pass like
  story 1.6's for FORM, reading `METHOD x.` ... `ENDMETHOD.` off the tokens,
  would add the lost methods and end a grammar method at its own `ENDMETHOD`.
  Gate 1's diff-quality check fails on this alone, so this candidate blocks
  Gate 1 and may belong in Tier 1.
- **T2-B Interface method declarations as entities** (Gate 1 census, #4432
  `zif_abapgit_popups`). `METHODS` in an `INTERFACE` are not entities, so a
  changed signature reads as the interface changing.
- **T2-C One entity per name in a chained `DATA:`** (Gate 1 census, #3185,
  #4432). A chained `DATA: a ..., b ... .` is one variable entity named after
  one member, so adding or renaming a member reads as a delete and an add, or
  as a different variable changing.
