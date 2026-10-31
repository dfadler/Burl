# Burl

A source-control repository visualiser written in Rust. It began as a rewrite of Gource, a C++ program, and is its own project from here on: it is never submitted back to Gource. The first release matches Gource's behaviour; breaking changes and new features come only after that is proven.

## Language

**Burl**:
The project and the program it produces: the Rust rewrite of Gource, and what it grows into.
_Avoid_: the port, Gource-rs, the Rust Gource

**Gource**:
The original C++ program (version 0.57) that Burl was derived from and is checked against.
_Avoid_: upstream, the old version

**Oracle**:
The original Gource, kept buildable so its outputs can be captured and compared with Burl's.
_Avoid_: reference build, baseline

**State-dump fork**:
A copy of Gource's sources that also writes its simulation state each frame, used only to check parity. It is not the oracle for byte-exact goldens.
_Avoid_: patched Gource, debug build

**Golden**:
An output captured from the oracle, stored in the repository, and used as the expected value in a test.
_Avoid_: snapshot, fixture (a fixture is an input such as a sample log)

**Parity**:
Burl produces the same parsed commits and the same simulation state as Gource for the same inputs and settings. Rendering only has to be visually equivalent, not pixel-identical.
_Avoid_: compatibility, identical output, same output

**Phase**:
One of the numbered stages (0 to 7) of the implementation order in `docs/rust-rewrite/discovery/rust-architecture-map.md`.
_Avoid_: milestone, step, "phase 1" for the first build session (call that build session 1)
