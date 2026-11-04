# G4 decision note: does `pcre2` ship by default?

Decision owner: the user (RELEASE.md step 3). This note gives the facts and a recommendation; it decides nothing. Row: G4 in [DEVIATIONS.md](DEVIATIONS.md).

## What differs

| | Default build (no feature) | `--features pcre2` |
|---|---|---|
| Engine for `--file-filter`, `--file-show-filter`, `--user-filter`, `--user-show-filter` | `regex` crate, Unicode off | PCRE2 (`pcre2` 0.2.11 over `pcre2-sys` 0.2.10), as Gource |
| Look-around, backreferences in a filter | Rejected at import: `invalid file-filter regular expression`, exit 1 (loud) | Work, like Gource |
| `--git-branch` check `^(?!-)...` | Done by hand (`valid_git_branch`, `settings.rs:458`), same result | PCRE2 |
| Internal log-parser patterns | `regex` always (no look-around; matches PCRE2 exactly) | unchanged |
| Pinned by | `deviation_g4_default_build_rejects_pcre2_only_filters` | `sim_gate_pcre2_filters`; CI leg `linux-pcre2` runs the whole fast tier with the feature |

Behavioural risk of the default: a user whose existing Gource filter uses look-around gets an error at start, not a silent wrong result. Plain patterns behave the same in both.

## Cost of shipping it

- **Build time.** Measured here (macOS, debug, clean scratch target dir, `PCRE2_SYS_STATIC=1`, `cargo build -p burl-util --features pcre2`): 24 s wall for the whole feature-enabled dependency set, of which the C build of PCRE2 is part. Release-profile and CI timings not measured.
- **C compiler at build time.** `pcre2-sys/build.rs` compiles the bundled PCRE2 C sources with `cc` unless it finds `libpcre2-8` through `pkg-config`. The release Dockerfile's build stage (`rust:1-slim-trixie`) has a C toolchain for linking; whether it has `pkg-config` and `libpcre2-dev` is not checked here.
- **System-library trap.** With `pkg-config` and a system `libpcre2-8` present, `pcre2-sys` links it dynamically (it is present on this Mac: `pkg-config --exists libpcre2-8` succeeds). A release tarball would then depend on a library the user may not have. The build script skips the probe when `PCRE2_SYS_STATIC` is set, so a shipped build would need `PCRE2_SYS_STATIC=1` in `release.yml` and `docker/release.Dockerfile`.
- **Binary size.** Not measured for a release build. The debug static archive is 1.4 MB (`libpcre2.a`, includes debug info); the release increase will be smaller.
- **Release image.** `docker/release.Dockerfile` runtime stage installs only `libegl1`, `libegl-mesa0` and `libgl1-mesa-dri`. With a static link the image needs nothing added. With a dynamic link it would need `libpcre2-8-0` (not checked whether the base already has it).
- **Licences.** Not verified here. `pcre2` is in `Cargo.lock`, so the CI `deny` job already sees the crates; the licence of the bundled upstream PCRE2 C sources (inside `pcre2-sys/upstream`) is not what `cargo deny list` reports for the crate, so a notice for them would need checking before it ships in `THIRD-PARTY-LICENSES.txt`.
- **Gate coverage.** The release gates (sim comparator, visual gate, image check) run on the default build. Shipping `pcre2` by default means the shipped binary is the `pcre2` leg's, so the heavy tier would have to run with the feature, or the "binary the gates tested is the binary that ships" rule (ADR 0001 item 4, #22) is not met for filters.

## Options

1. **Default build ships without `pcre2` (current state).** Zero new C code; the gates already test it. Cost: Gource filters with look-around fail loudly; G4 stays a documented Gap with its fix trigger ("a user reports a filter that needs it"). A user who needs it builds with `--features pcre2`.
2. **Ship with `pcre2` statically linked.** Closes G4 and the deviation row. Cost: set `PCRE2_SYS_STATIC=1` in both release builds, move the heavy tier (or at least the sim and filter gates) to the feature build, check the bundled-source licence notice.

## Recommendation

Option 1 for 0.58.0. The default is loud, not silently wrong; no pinned fixture needs the feature outside its own leg; and option 2 changes what the heavy tier must test, which is a bigger release-process change than the Gap justifies. Revisit when the first user report arrives, then take option 2 with the changes above in the same PR. If option 1 is chosen, the register edit is: G4 stays a Gap and the fix trigger loses "the release build decides", and RELEASE.md step 3 drops "decide G4".
