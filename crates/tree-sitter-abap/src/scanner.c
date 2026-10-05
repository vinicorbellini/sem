// External scanner for sem's fork of tree-sitter-abap.
//
// It has one token, `bol_comment`: a `*` in the first column of a line, to
// the end of that line. ABAP reads a `*` anywhere else as an operator (or
// `SELECT *`), so the grammar cannot say "a `*` and the rest of the line"
// with a plain token: that also eats `lv = lines( lt ) * 2.` from the `*`
// on, period included. Only the column tells the two apart, and a regular
// token cannot see the column.
#include "tree_sitter/parser.h"

// No <wctype.h>: sem-plugin builds this for wasm32 with only the few libc
// headers tree-sitter-language provides.
static bool is_space(int32_t c) {
  return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' || c == '\v';
}

enum TokenType {
  BOL_COMMENT,
};

void *tree_sitter_abap_external_scanner_create(void) { return NULL; }

void tree_sitter_abap_external_scanner_destroy(void *payload) { (void)payload; }

unsigned tree_sitter_abap_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload;
  (void)buffer;
  return 0;
}

void tree_sitter_abap_external_scanner_deserialize(void *payload, const char *buffer,
                                                   unsigned length) {
  (void)payload;
  (void)buffer;
  (void)length;
}

bool tree_sitter_abap_external_scanner_scan(void *payload, TSLexer *lexer,
                                            const bool *valid_symbols) {
  (void)payload;
  if (!valid_symbols[BOL_COMMENT]) return false;

  // Column 1 is the character after a line end. Read that off the
  // whitespace skipped here: get_column() has been seen to answer 0 for a
  // `*` that follows a statement on the same line. With no whitespace
  // before it, the `*` touches the previous token, so it is column 1 only
  // at the start of the file, where get_column() is reliable.
  bool skipped = false;
  bool after_line_end = false;
  while (is_space(lexer->lookahead)) {
    after_line_end = lexer->lookahead == '\n';
    skipped = true;
    lexer->advance(lexer, true);
  }

  if (lexer->lookahead != '*') return false;
  if (skipped ? !after_line_end : lexer->get_column(lexer) != 0) return false;

  while (lexer->lookahead != '\n' && !lexer->eof(lexer)) lexer->advance(lexer, false);
  lexer->mark_end(lexer);
  lexer->result_symbol = BOL_COMMENT;
  return true;
}
