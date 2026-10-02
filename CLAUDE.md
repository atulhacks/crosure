# Crosure contributor rules

- Keep files under ~300 lines; split into modules/components.
- Every public Rust item has a doc comment; library functions get doctests where practical.
- Every exported TypeScript function gets JSDoc.
- No `unwrap`/`expect`/`panic` outside tests (clippy enforces it).
- No banner comments like `// ------ Section ------`.
- Every user/agent action that touches a binary must go through the recorder.
- Never commit live malware; `samples/` holds benign fixtures only.
