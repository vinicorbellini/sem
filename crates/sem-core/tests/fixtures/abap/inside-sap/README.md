# Inside-SAP drop zone

Files an agent with access to a real SAP system will add here later. Nothing
else lives in this directory until then.

- `whereused.json`: `sapcli whereused` output for the 30 study methods, as a JSON object keyed by the method's `CLASS=>METHOD` (or `CLASS~METHOD` for interface methods), each value the array of `{ "object": "<abapGit name>", "type": "<TADIR type>", "include": "<include or method>", "line": <int> }` references sapcli returns.
- `parse-error-census.json`: a JSON object `{ "scanned": <int>, "files": [ ... ] }`, where `files` holds one `{ "file": "<abapGit file name>", "errors": <int>, "first_error": { "line": <int>, "column": <int>, "snippet": "<source text>" } }` per source file the tree-sitter grammar parses with errors (clean files are omitted) and `scanned` is the total number of files parsed.
- `grammar-bugs/<construct>.abap`: one minimal, syntactically valid ABAP snippet per grammar failure, named after the construct (for example `inline-declaration-in-let.abap`), starting with a `* construct: ...` comment line and a second comment line stating what the grammar reports.
