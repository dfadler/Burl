# Gource Rust Rewrite

A rewrite of Gource, a source-control repository visualiser, from C++ to Rust. The first release matches the C++ program's behaviour; breaking changes and new features come only after that is proven.

## Language

**Parity**:
The Rust program produces the same parsed commits and the same simulation state as the C++ program for the same inputs and settings. Rendering only has to be visually equivalent, not pixel-identical.
_Avoid_: Compatibility, identical output, same output
