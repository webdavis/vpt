Complete docs/vpt-remaining-work.md using its linked product specifications. Work only within VPT.

Standards:
- TDD (test-driven development): failing behavioral test, minimal implementation, refactor.
- SOLID (single responsibility, open/closed, substitutability, interface segregation, dependency inversion).
- DRY (don’t repeat yourself), without speculative abstractions.
- Self-documenting code; comments only for non-obvious intent or safety.
- Production files should stay within 150 lines, never exceed 250. Separate test files. Split by cohesive domain responsibility.
- Idiomatic safe Rust: explicit ownership, typed errors, deterministic cleanup, bounded resources, and preserved privacy and durability.
- Unsafe is rare and fenced off
  - Keep each unsafe block as small as possible, isolated in its own small module.
  - Every single block requires a one- or two-line // SAFETY: comment explicitly proving why undefined behavior is impossible.

Work on multiple features at once if you can.

Do NOT run reviews. Let the tests and compile drive the quality.

Continue until finished or genuinely blocked. Keep updates brief.

