# Independent fixture source assets

These XML and document templates were transcribed from the deterministic generators at commit `750c4674df79c3a0d7f9bd014462ace525e8dcbc`. They are generator source assets, not engine output or test successes. `${0}`, `${1}`, etc. are positional substitutions made in one pass by Rust. Callers escape document text before insertion; relationship and package order remains explicit in the Rust generator.

common / context / conditions / office / issues correspond to the former fixture generators. documents.json contains the text/code source documents, including the intentionally searchable sample.py. No Python interpreter is used by the current generator or validation workflow. For changing corpus content, update the independent case contract, inventory checks, documentation and migration comparison together.
