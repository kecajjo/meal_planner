### Review Philosophy

* Only comment when you have HIGH CONFIDENCE (>80%) that an issue exists
* Be concise: one sentence per comment when possible
* Focus on actionable feedback, not observations
* When reviewing text, only comment on clarity issues if the text is genuinely confusing or could lead to errors.

### Correctness Issues

* Logic errors that could cause panics or incorrect behavior
* Race conditions in async code
* Resource leaks (files, connections, memory)
* Off-by-one errors or boundary conditions
* Incorrect error propagation (using `unwrap()` inappropriately)
* Optional types that don’t need to be optional
* Booleans that should default to false but are set as optional
* Error context that doesn’t add useful information
* Overly defensive code with unnecessary checks
* Unnecessary comments that restate obvious code behavior

### Architecture & Patterns

* Code that violates existing patterns in the codebase
* Missing error handling (should use `anyhow::Result`)
* Async/await misuse or blocking operations in async contexts
* Improper trait implementations

### Skip These (Low Value)

Do not comment on:

* Style/formatting (rustfmt, prettier)
* Clippy warnings
* Test failures
* Missing dependencies (npm ci covers this)
* Minor naming suggestions
* Suggestions to add comments
* Pedantic text accuracy unless it affects meaning

### Response Format

1. State the problem (1 sentence)
2. Why it matters (1 sentence, if needed)
3. Suggested fix (snippet or specific action)

Example:
This could panic if the vector is empty. Consider using `.get(0)` or adding a length check.

### When to Stay Silent

If you’re uncertain whether something is an issue, don’t comment.