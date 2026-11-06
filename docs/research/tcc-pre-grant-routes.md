# TCC pre-grant routes for Screen Recording and Accessibility

Issue: dfadler/Burl#141 (part of #136). Researched 2026-11-05. Primary sources only.

Legend: **VERIFIED** = read in the owning source this session. **INFERENCE** = follows from verified facts but not stated by the source. **NOT VERIFIED** = no primary source found.

## Short answer

| Route | Accessibility | Screen Recording | Needs |
|---|---|---|---|
| PPPC / MDM profile | Can pre-grant (`Allow`) | **Cannot pre-grant, only deny** (standard-user self-approval is the most it offers) | MDM enrollment; deprecated in macOS 27 |
| Direct TCC.db write | Works on runner-images VMs | Works on runner-images VMs, plus extra `replayd` step on macOS 15+ | `sudo`; SIP/FDA requirements undocumented by Apple; changed on macOS 27 |
| runner-images image bake | Pre-granted for specific binaries | Pre-granted for specific binaries | Only on GitHub-hosted macOS images, not self-hosted |

## 1. PPPC / MDM profile

- **VERIFIED** Payload `com.apple.TCC.configuration-profile-policy`; macOS 10.14+, device channel only, no user channel, `userapprovedmdm: true`, `allowmanualinstall: false`.
  Source: https://github.com/apple/device-management/blob/release/mdm/profiles/com.apple.TCC.configuration-profile-policy.yaml
- **VERIFIED** `Accessibility`: "Specifies the policies for the app via the Accessibility subsystem." Supports `Authorization: Allow`. Deprecated in macOS 27.0: "the device shows a non-blocking notification for each application when this setting is applied" and the user can change it; replacement is the `Privacy` key of declarative `com.apple.configuration.app.settings`. (same YAML, `Accessibility` key)
- **VERIFIED** `ScreenCapture` (macOS 10.15+): "A profile can't grant access to the contents; it can only deny it." (same YAML)
- **VERIFIED** `Authorization: AllowStandardUserToSetSystemService` is "only valid for the `ListenEvent` and `ScreenCapture` services". It lets a non-admin user flip the toggle themselves; it is not an unattended grant. `Authorization` introduced in macOS 11.0. (same YAML)
- **VERIFIED** Apple's deployment guide words Screen recording as deny ("Deny specified apps access to capture (read) the contents of the system display") and Accessibility as allow ("Allows specified apps to control the Mac via Accessibility APIs").
  Source: https://support.apple.com/guide/deployment/privacy-preferences-policy-control-payload-dep38df53c2a/web
- **VERIFIED, sources differ** The guide says "Supervision is required if you apply this payload using a device management service" and approval is "Requires user approval". The YAML says `userapprovedmdm: true` and does not mention supervision. Treat supervision as a possible requirement; check against the MDM actually used.
- **VERIFIED** The payload is installable only via a device management service (`allowmanualinstall: false`), so a CI runner not enrolled in MDM cannot use this route.

## 2. Direct TCC.db edits

- **VERIFIED** Apple does not support it. DTS (Quinn "The Eskimo!"), Oct 2023: "the location and format of that is an implementation detail, and thus not something that DTS supports." Source: https://developer.apple.com/forums/thread/740055. That reply does not mention SIP or Full Disk Access.
- **NOT VERIFIED** "System TCC.db is protected by SIP; user TCC.db needs Full Disk Access for the writer." No Apple document states it. Only non-Apple posters in https://developer.apple.com/forums/thread/730043 report it (system DB write-protected under rootless; one edited the user DB from a Terminal that already had Full Disk Access to avoid disabling SIP). Apple staff in that thread did not address SIP, ScreenCapture or Accessibility.
- **VERIFIED** GitHub's image builds write both databases with `sqlite3`: `sudo sqlite3 "/Library/Application Support/com.apple.TCC/TCC.db"` for the system DB and plain `sqlite3 "$HOME/Library/Application Support/com.apple.TCC/TCC.db"` for the user DB, via `INSERT OR IGNORE INTO access VALUES(...)`. Source: `images/macos/scripts/helpers/utils.sh` (`configure_system_tccdb`, `configure_user_tccdb`) in https://github.com/actions/runner-images
- **VERIFIED** Neither `configure-tccdb-macos.sh`, `configure-tccdb-macos-27.sh` nor the macOS 26 arm64 template contains `csrutil`.
- **INFERENCE** Those sudo writes succeed at image build time, so SIP is off or not blocking in that Anka VM build context. The scripts do not say which. Do not assume a stock Mac with SIP on allows this.
- **Version notes**
  - macOS 14 arm64, 15 arm64, 26 arm64 and 26 Intel templates: **VERIFIED** they run `configure-tccdb-macos.sh`. (macOS 14/15 Intel templates exist in the repo; not individually checked.)
  - macOS 27: **VERIFIED** `xcode-27.arm64.anka.pkr.hcl` runs `configure-tccdb-macos-27.sh`. For major version 27 the user DB is no longer under `$HOME`; the script `sudo find`s `/private/var/containers/Data/ProtectedSystem/.../Data/Library/Application Support/com.apple.TCC/TCC.db` and, if several match, picks the one `tccd` has open (via `lsof`). Both DBs are written with `sudo sqlite3` and an explicit column list.
  - A forum post that the user DB moved into ProtectedSystem on a macOS 27 beta (https://developer.apple.com/forums/tags/privacy/?page=2) matches the script but is a user report: **NOT VERIFIED** as an Apple statement.
  - The `access` insert differs between scripts (positional `VALUES(...)` vs named columns incl. `pid`, `boot_uuid`, `last_reminded`). **VERIFIED** from script text.

## 3. actions/runner-images `configure-tccdb-macos.sh`

Source: https://github.com/actions/runner-images/blob/main/images/macos/scripts/build/configure-tccdb-macos.sh (latest commit 1b21701a, 2026-08-03, PR #14489).

- **VERIFIED** Templates run it with `sudo` (see 2 for which images).
- **VERIFIED** System DB rows grant `kTCCServiceAccessibility` and `kTCCServiceScreenCapture` (auth_value 2) to `/bin/bash`, `/opt/hca/hosted-compute-agent`, `/usr/local/opt/runner/provisioner/provisioner`; Accessibility also to `/usr/bin/osascript`, `/opt/hca/start_hca.sh`, `sshd-keygen-wrapper`, `com.apple.Terminal`, `com.apple.dt.Xcode-Helper`. The user DB repeats the pattern (ScreenCapture also for `osascript` and Terminal).
- **VERIFIED** Every job gets a fresh VM, so grants are baked into the image, not applied per job.
- **VERIFIED** macOS 15+ extra step: a `kTCCServiceScreenCapture` row is "not enough". The script also writes `~/Library/Group Containers/group.com.apple.replayd/ScreenCaptureApprovals.plist` with a far-future date (`3024-01-01`) for `/opt/hca/hosted-compute-agent`, then `killall cfprefsd`, to suppress the recurring "bypass the system private window picker" alert that takes focus and breaks headed UI tests. Keyed by the responsible process. The replayd file format is private; **NOT VERIFIED** by any Apple source.
- Relevance to Burl: grants cover the hosted runner agent and `bash`. Self-hosted or local runners are not covered, and neither is a binary Burl builds.

## Open items

- Apple documentation of SIP/FDA scope for each TCC.db: none found.
- Whether SIP is actually off in the Anka build VM: not shown in the scripts.
- The macOS 27 declarative `Privacy` key schema was not read; only the deprecation pointer was.
