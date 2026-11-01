# Phase 1: workspace skeleton and log-format parsers

Status: `cargo build` and `cargo test` pass; every `--output-custom-log` golden in
`discovery/golden/custom-log/` (five fixtures, the stdin variant and the unterminated-stdin
case) matches byte for byte, as do `help-short.txt`, `help-long.txt`, the `--version` error
and every `rc=` row of `golden/exit-codes.txt` except the SDL probe. (True of the code at the time of writing, with the program name normalised for Burl. After the D5 help-text fixes the Rust help output deliberately differs from the Gource goldens on the user-friction and dir-name-position lines; the harness then compares against the golden with the expected diff under `discovery/golden/expected-diffs/` applied, which does not exist yet.) The 28 `parseDateTime`
vectors pass with the zone pinned, and again through the binary with `TZ=Pacific/Auckland`.

## What is here

| Crate | Contents |
|---|---|
| `gource-util` | `regex` wrapper (unanchored, byte mode, empty subject never matches, trailing non-participating groups dropped), `strhash` (`stringHash`, `colourHash`, `vec2Hash`, `vec3Hash`), `math::normalise`, `utf8::replace_invalid`, `timeparse` (`parseDateTime`, `timegm`, `mktime` with normalisation), `cstr::atoll/atoi` |
| `gource-conf` | the full `arg_types` / `arg_aliases` / `conf_sections` tables and the two help texts (`include_str!`) |
| `gource-formats` | `RCommit`/`RCommitFile`, `SeekLog` (istream flag model) and `StreamLog`, the `CommitLog` trait with `checkFormat`/`nextCommit`/`isFinished`, nine parsers with verbatim regexes and command strings, `logmill::fetch_log` with the C++ detection order, `findRepository` and error selection, VCS log generation via `sh -c "<cmd> > tmp"` |
| `gource` | `main.cpp` flow up to `--output-custom-log`; `-h`/`-H`; the `*-command` printers |
| `gource-sim`, `gource-render`, `gource-gl` | empty |

Regex engines: every internal pattern compiles on `regex::bytes` with Unicode off (a unit
test compiles the whole list). Real PCRE2 is behind the `pcre2` cargo feature of
`gource-util` for the four user filters and the `--git-branch` look-ahead, which phase 2
imports; `cargo test -p gource-util --features pcre2` builds and passes here against the
system `libpcre2-8`. `Regex::new_user` picks PCRE2 when the feature is on.

## Decisions taken (ledger D1-D5)

Only D5 touches this phase. Decided 2026-10-31: the known C++ bug is fixed, so an explicit
`--log-format cvs-exp` selects the CVS-EXP parser (the code currently still matches `cvs`,
which settings reject, so it yields "unsupported log format"; the fix and its test are
pending in the code). Gource still fails it, so this is a documented deviation test.
D1-D4 do not reach the log layer.

## Deliberately deferred

- **Settings import**: only `--log-format`, `--output-custom-log`, the positional path and the
  command-line printers do anything. Every other option is consumed exactly as `parseArgs`
  would (so `unknown option` and value swallowing match) and then dropped: `--load-config`,
  `--save-config`, filters, dates, `--hash-seed`, `--git-branch`, `--author-time` have no
  effect yet, and the positional `.conf/.cfg/.ini` detection does not run. Validation
  messages for options other than `log-format` and the stdin check do not fire.
- **Without `--output-custom-log`** the binary prints a non-C++ message and exits 1; there is
  no display path.
- **`StreamLog`** reads stdin blocking. C++ uses `O_NONBLOCK` and can lose the first half of
  a line split across pipe writes (misc-traces.md sec. 2); that bug is not reproduced. The
  100 ms startup poll is replaced by the blocking first read.
- **`SeekLog`** over 100 MiB: C++ streams from disk; Rust still reads the file whole (and,
  like C++, skips the NUL truncation for it). `getCommitAt`/`getPointer` are not ported.
- **Logmill thread**, the `start_timestamp` skip loop and `stop_timestamp` handling belong
  to the sim phase.
- **`mktime` edge cases**: ambiguous local times take the earlier instant, nonexistent ones
  (DST gap) use the offset in force before the gap. No golden covers either.
- **TinyXML fidelity**: `GetText` first-child and whitespace condensing are ported; CDATA is
  condensed too (TinyXML keeps it verbatim) and an undefined entity (`&foo;`) fails the entry
  where TinyXML would keep it literally. Entries with non-UTF-8 bytes are rejected by
  `roxmltree` instead of being filtered later.
- **Resource dir** for the `hg` command defaults to `data/`; the C++ build bakes
  `SDLAPP_RESOURCE_DIR` in.
- `apache_entry_agent` and `apache_hostname_parts` are not transcribed; their results are
  unused in C++.
- Git's `--no-show-signature` probe (`git --version`) runs only when the command string is
  needed, not on every `GitCommitLog` construction as in C++; output is identical.
- `RCommit::username` is `Vec<u8>` (raw during parsing, valid UTF-8 after `postprocess`)
  because the custom and hg grouping compares the raw bytes.
- **SIGPIPE**: Rust ignores it, so `--output-custom-log - | head` writes the whole log and
  exits 0 where the C++ binary dies on the broken pipe (the ledger lists PPM/SIGPIPE as
  unexercised anyway).
- **PCRE2 in CI**: `cargo test` with default features never exercises the PCRE2 engine; run
  `cargo test -p gource-util --features pcre2` as well, and turn the feature on in phase 2
  when the filters are imported.

## Rename to burl (2026-10-31)

The binary crate was renamed from `gource` to `burl` (`crates/burl`, binary `burl`) after this
document was written; the `gource` row in the table above and other references to the `gource` binary mean that crate. The help text now
says Burl, and `crates/burl/tests/golden.rs` compares output with the Gource goldens through a
narrow program-name normalisation (see `docs/rust-rewrite/discovery/LEDGER.md`, Decisions).
