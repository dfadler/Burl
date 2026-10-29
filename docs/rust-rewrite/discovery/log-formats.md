# Gource log formats and log-loading pipeline (discovery)

All paths relative to repo root. Cites are `path:line`.
"commit" = `RCommit` (`src/formats/commitlog.h:42`): `timestamp: time_t`, `username`, `files: list<{filename, action: String, colour: vec3}>`.

## 1. Pipeline overview

1. `RLogMill` spawns a thread (`src/logmill.cpp:37`); `run()` (`logmill.cpp:65`) calls `fetchLog(gGourceSettings.log_format)`.
2. `fetchLog` (`logmill.cpp:190`): if `log_format` empty and path != `"-"` and path is a directory, `findRepository` (`logmill.cpp:163`) canonicalises the path and walks up parents; per directory the first hit in order `.git` (dir OR regular file) > `.hg` > `.bzr` > `.svn` (`logmill.cpp:171-174`); nearest ancestor wins; `logfile` is rewritten to the repo root and `log_format` set. None found: falls into auto-detect (all fail for dirs) -> "directory not supported" (`logmill.cpp:120`).
3. Explicit format (`logmill.cpp:213-262`): tries only that class; `git` tries `GitCommitLog` then `GitRawCommitLog` (`logmill.cpp:216-224`). Failure of `checkFormat()` returns null.
4. Auto-detect order for files/stdin (`logmill.cpp:272-334`): **git, hg, bzr, git-raw, cvs-exp, svn, cvs2cl, custom, apache**. First `checkFormat()` success wins.
5. If `start_timestamp != 0` (`logmill.cpp:85`): read validated commits until `timestamp >= start_timestamp`, then `bufferCommit` it. Loop is `while(!shutdown && !isFinished())`; on stdin `isFinished()` is always false so it spins until a commit is found.
6. Errors (`logmill.cpp:98-123`): `SeekLogException` -> "unable to read log file"; `SDLAppException` -> its message; null clog + dir + format given -> "failed to generate log file" (+ " for the specified time period" if start/stop timestamp set); dir without format -> "directory not supported"; file -> "unsupported log format (you may need to regenerate your log file)".
7. **Bug to preserve or fix consciously**: `--log-format cvs-exp` is accepted by settings (`src/gource_settings.cpp:784`) but `fetchLog` only matches the string `"cvs"` (`logmill.cpp:238`), and settings reject `"cvs"` (`gource_settings.cpp:779`). Explicit `cvs-exp` therefore yields "unsupported log format"; auto-detect of a cvs-exp file still works. Accepted values: git, cvs-exp, cvs2cl, svn, custom, hg, bzr, apache (`gource_settings.cpp:783-790`; help lists fewer, `:99`). `--log-format` is mandatory for stdin (`gource_settings.cpp:1636-1638`).
8. Consumption (`src/gource.cpp:1131-1185`): `nextCommit` (validated); on failure non-seekable -> break (retried next frame), seekable -> continue (resync line by line). Commit with `timestamp > stop_timestamp` (when nonzero) sets stop_position_reached and is discarded (`gource.cpp:1148`). Queue refilled while `queue.empty() || (back.timestamp <= currtime && size < 100)` (`gource.cpp:1137`, max at `:919`). Empty first read throws "no commits found". **No sorting anywhere in the log layer**: commits processed in file order.
9. `--output-custom-log` (`gource.cpp:170-216`) re-emits any log as custom format: `"%lld|%s|%s|%s\n"` = `timestamp|username|action|filename` per file (filename with leading `/`, no colour), validated `nextCommit`; `-` = stdout.

## 2. Common commit logic (`src/formats/commitlog.cpp`)

| Item | Behaviour | Source |
|---|---|---|
| Constructor | `"-"`: if first-char check passes (`cin.peek()`) -> `StreamLog`, non-seekable. Else `stat`; directory sets `is_dir`; regular file: peek first char vs expected, if OK `SeekLog`, seekable. Mismatch leaves `success=false`. `firstChar=-1` = no check. | commitlog.cpp:40-83, 112-122 |
| First-char expectations | git `'u'`, gitraw `'c'`, svn `'<'`, cvs2cl `'<'`; hg, bzr, cvs-exp, custom, apache: none | git.cpp:130, gitraw.cpp:34, svn.cpp:49, cvs2cl.cpp:37 |
| `checkFormat` | `success` required; parses one commit with `validate=false`; on success seekable -> `seekTo(0.0)` + clear `lastline`; non-seekable -> mark that commit buffered. Returns the parse result. | commitlog.cpp:124-143 |
| `nextCommit(commit, validate=true)` | returns buffered commit if any; else reset, `parseCommit`, `postprocess()` (UTF-8 filter username), then `isValid()` if validate. | commitlog.cpp:216-236 |
| `isValid` | any `user-filter` match -> reject; **every** `user-show-filter` must match (AND) else reject; finally `!files.empty()`. Unanchored search on UTF-8-filtered username. Zero-file commits are silently skipped. | commitlog.cpp:357-384 |
| `addFile` filters | `file-filter`: any match drops the file; `file-show-filter`: every one must match (AND). Applied to the **raw** filename (before leading `/`, before UTF-8 filtering). | commitlog.cpp:325-351 |
| `RCommitFile` ctor | `filename = filter_utf8(filename)`; if `filename[0] != '/'` insert `/` (also for empty name). Nothing else: no backslash conversion, no `//` collapse, no `./` strip; trailing `/` kept (directory marker). | commitlog.cpp:290-301 |
| UTF-8 | `utf8::replace_invalid(..., '?')` (utf8-cpp: each invalid sequence -> `?`); on exception whole string = `"???"`. Applied to filenames and usernames only. | commitlog.cpp:24-36, 354 |
| Default file colour (`fileColour`) | `slash=rfind('/')`, `dot=rfind('.')`; if dot found, `dot+1 < len`, and (no slash or slash < dot): `colourHash(substr(dot+1))`, else white `(1,1,1)`. Computed on raw filename. NB `RFile::colourize` recomputes from its own ext (`src/file.cpp:128-137`, incl. `file_extension_fallback`); `cf.colour` feeds the `RFile` ctor and `ModifyAction` (`gource.cpp:1005,1285`). | commitlog.cpp:307-323 |
| Action semantics (consumer) | `"D"` remove, `"A"` create, anything else (M, R, T...) modify. Filename ending `/`: ignored unless action `D`, which removes all files under matching dirs. | gource.cpp:1199-1204, 1279-1286 |
| Line reading | `getline` on `\n`; strip one trailing `\r` (CRLF handled) | seeklog.cpp:73-75, 187-190 |
| `getNextLine` (commit layer) | returns push-back `lastline` first (custom/hg), else underlying log | commitlog.cpp:172-180 |
| `findNextCommit(c, n)` | up to n `nextCommit` calls | commitlog.cpp:197-209 |
| `getCommitAt(percent)` | seekable only: save pointer+`lastline`, `seekTo`, `findNextCommit(500)`, restore (does not restore `buffered`/`current_percent`). Slider hover date. | commitlog.cpp:153-170 |
| `isFinished` | `seekable && logf->isFinished()`; non-seekable never finished | commitlog.cpp:238-242 |
| Temp file | non-Windows `mkstemp("/tmp/gource-XXXXXX")`, removed in dtor | commitlog.cpp:253-286, 85-91 |
| Command exec | `system(cmd)`; `requireExecutable` is Windows-only; rc != 0 -> generation fails (null) | commitlog.cpp:93-109 |
| Windows | git: no single quotes; hg: `'` -> `"` | git.cpp:28, hg.cpp:44 |

Command date inputs: `start_date = strftime("%Y-%m-%d", localtime(start_timestamp))`; `stop_date` same but **+1 day when stop time has any nonzero h/m/s** (`gource_settings.cpp:1271-1311`). VCS-side filtering is date-granular; exact filtering by timestamp: start in logmill, stop in `readLog`. Custom/stdin logs get only the timestamp filtering.

### parseDateTime (`src/core/settings.cpp:284-363`)
Used by `--start-date/--stop-date` and the custom-log timestamp field. PCRE2:
`^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{1,2}):(\d{2})(?::(\d{2}(?:\.\d+)?))?)?(Z| ?([+-])(\d{1,2})(?::?(\d{2}))?)?$`
- No time -> 00:00:00; fractional seconds parsed (atoi truncates).
- `Z` -> `mktime_utc`; explicit `+/-H[H][[:]MM]` -> `mktime_utc(..) - offset` (group 8 sign); neither -> local `mktime`. `mktime_utc` = set `TZ=UTC`, `mktime`, restore (`src/core/timezone.cpp:41`).
- Code branches on `results.size()` (trailing unmatched groups dropped, see sec. 3): `==7 && [6]=="Z"`, `>=9` for offset. Reimplement with named groups.

## 3. Regex wrapper (`src/core/regex.cpp`) and PCRE2 features

- Compiled with PCRE2, flags 0: **no UTF, no UCP, no caseless**. Byte-wise matching; `\d \w \s` ASCII; `.` excludes `\n`; single-line `^ $`.
- `match(str, results)` (`regex.cpp:149`) is an **unanchored search**. `results` = groups 1..rc-1 where rc = highest participating group + 1: **trailing non-participating groups are omitted**; middle ones become `""` (`regex.cpp:195-208`). Parsers rely on `results.size()` (custom colour `>=5`, apache `!=4/!=8/<5`, parseDateTime).
- **Empty subject never matches**: `matchOffset` returns -1 if `offset >= str.size()` (`regex.cpp:174`), so even `^$` fails on `""` (e.g. user filter vs empty username).
- `replace/replaceAll` support `$N` in replacement (`regex.cpp:75-146`; used outside log code).
- Invalid pattern: constructor throws `RegexCompilationException` unless `test=true`; user filters compiled with `test=1` -> "invalid file-filter regular expression" (`gource_settings.cpp:1525-1531`).
- Features in log parsers: anchors, negated classes `[^|]`, `\d`, `\s` in class, `{4}`/`{6}`, non-capturing groups, lazy `.*?` (apache), optional groups, escapes `\[ \] \| \(`, raw byte escape `\xEF\xBB\xBF` (BOM), unescaped literal `}`. No backrefs/lookaround in log parsers.
- User filters (`--file-filter`, `--user-filter`, `--file-show-filter`, `--user-show-filter`; multi-value, `gource_settings.cpp:315-318`) are arbitrary PCRE2. `--git-branch` validation uses **negative lookahead**: `^(?!-)[/\w.,;_=+{}\[\]-]+$` (`gource_settings.cpp:743`).
- Rust: `regex::bytes::Regex` with `(?-u)` covers every parser pattern (byte escapes need bytes API or manual BOM strip). For user filters and git-branch use `fancy-regex` or `pcre2` crate for fidelity.

## 4. SeekLog / StreamLog (`src/core/seeklog.cpp`, `.h`)

| Aspect | Behaviour | Source |
|---|---|---|
| `SeekLog` load | open binary+ate for size; if `file_size > 104857600` (100 MiB, `gSeekLogMaxBufferSize`) keep the `ifstream`; else read whole file into `char[]` and build `istringstream(std::string(buf))` — **construction from C string truncates at the first NUL byte**. Open failure -> `SeekLogException` -> "unable to read log file". | seeklog.cpp:35, 100-151 |
| `getNextLine` | if eof/fail `clear()` first; `getline`; strip one `\r`; return false if `fail()`; `current_percent = (float)tellg()/file_size`. Final line without trailing `\n` sets eof (returned true) and `tellg()` returns -1 -> percent = -1/size (edge). | seeklog.cpp:183-204 |
| `isFinished` | `fail() || eof()`. A file ending in `\n` is not "finished" until a read fails. | seeklog.cpp:219-229 |
| `seekTo(percent)` | `offset = (streampos)(percent * file_size)` (float multiply, precision loss on big files); `clear()`+`seekg`; **if offset != 0 discard the next line** to resync. | seeklog.cpp:161-181 |
| `getPercent` | `current_percent` of last successful line (uninitialised before first read; treat as 0.0) | seeklog.cpp:157 |
| `StreamLog` (stdin) | `fcntl(STDIN, O_NONBLOCK)` (failure -> permanent "finished"); `getline`; strip `\r`; **returns false if stream is fail/eof after read, so a final unterminated line is dropped**; if finished at entry, `clear()` to retry later (non-blocking: "no data yet" == failure). Windows: `PeekNamedPipe`. | seeklog.cpp:37-98 |
| Generated logs | VCS output -> temp file -> `SeekLog` (seekable) | git.cpp:191 |
| stdin | `seekable=false`: no slider, no start-position, no loop, no `getCommitAt` | commitlog.cpp:48-59, gource.cpp:1062, 1707 |

### Position options
| Option | Rules | Behaviour | Source |
|---|---|---|---|
| `--start-position F` / `-p` | float, 0<F<1 else error; `random` = `srand(time(0)); (rand()%1000)/1000.0f` | after log loads, if >0: `Gource::seekTo(F)` = `reset()` + `commitlog->seekTo(F)` (seekable only) | gource_settings.cpp:1314-1328; gource.cpp:1067-1078, 1604-1606 |
| `--stop-position F` | 0<F<=1 | stop when seekable and (`is_finished` or `getPercent() >= F`) | gource_settings.cpp:1330-1338; gource.cpp:1175 |
| `--stop-at-end` | bool | stop when `isFinished()` | gource.cpp:1172 |
| `--loop` + `--loop-delay-seconds` | | queue empty, seekable, idle >= delay: `first_read=true; seekTo(0.0); readLog()` | gource.cpp:1707-1713 |
| `--start-date/--stop-date` | parseDateTime | see sec. 2 | |
| Slider drag | `seekTo(percent)` | same | gource.cpp:495 |

## 5. stringhash (`src/core/stringhash.cpp`)

```cpp
int gStringHashSeed = 31;                        // --hash-seed N (gource_settings.cpp:1062); random 1..10000 on recolour (gource.cpp:1555)
int stringHash(const std::string& str) {
    int val = 0; int n = str.size();
    for (int i = 0; i < n; i++)
        val = val + str[i] * (gStringHashSeed ^ (n - i));   // XOR (not power); str[i] is SIGNED char
    if (val < 0) val = -val;
    return val;
}
vec3 colourHash(str) {                           // :71-83
    hash = stringHash(str); if (hash == 0) hash++;
    r = (hash/7) % 255; g = (hash/3) % 255; b = hash % 255;   // ints, truncating division
    return normalise(vec3(r, g, b));            // unit-length vector, not /255
}
vec2Hash: x=((hash/7)%255)-127, y=((hash/3)%255)-127 -> normalise(vec2)
vec3Hash: same x,y; z = hash%255 -> normalise(vec3)
```
Rust fidelity: bytes as `i8`; 32-bit signed arithmetic with `wrapping_add/mul` (C++ overflow wraps in practice); `-val` of `i32::MIN` stays MIN (`wrapping_neg`), after which `/` and `%` truncate toward zero and `%` may be negative. `normalise` is declared in `src/core/vectors.h:52-54` (zero-vector behaviour not examined). Users: file-extension colour (`file.cpp:137`, `key.cpp:61`, `commitlog.cpp:315`), user colour (`user.cpp:175`), directory placement via `vec2Hash` (`dirnode.cpp:788`).

## 6. Formats

"gen" = ctor runs a command when given a directory and no log file. All gen: temp file + `system("<cmd> > tmp")`. Commit grouping is described per format.

### 6.1 git (`src/formats/git.cpp`) - default, auto-detect first
First char `'u'` (`:130`). Command (`:88-128`), run with **cwd = repo dir** (chdir, restored), `"%s > %s"` in 2048-byte buffer (overflow = failure):
```
git log --reverse --raw --encoding=UTF-8 --no-renames [--no-show-signature] --pretty=format:user:%aN%n%ct [--since YYYY-MM-DD] [--until YYYY-MM-DD] [BRANCH]
```
- `%at` instead of `%ct` with `--author-time`.
- `--no-show-signature` if git version unknown (major==0) or >2 or (==2 and minor>=10). Version from `git --version > tmp` with regex `([0-9]+)(?:\.([0-9]+))?(?:\.([0-9]+))?` (`:36`), cached.
- `--git-branch` value appended verbatim (validated, sec. 3). `--git-log-command`/`--log-command git` prints the string (`gource_settings.cpp:555-561`).
- gen condition: `<dir>/.git` exists as dir or regular file (`:154-159`).

Parse (`:198-248`), per commit, lines until an empty line or EOF:
| Line | Handling |
|---|---|
| `user:` prefix | username = `substr(5)`; next line = timestamp `atol`; `0` (or non-numeric) -> return false; continue |
| other line, no username yet | return false |
| file line | `tab=find('\t')`; absent / at 0 / last char -> skip. `status = line[tab-1]` (single char), `file = substr(tab+1)`; empty -> skip; if starts and ends with `"` strip them (size<=2 -> skip). **No unescaping of git C-quoted `\NNN` octal** |
| end | true iff username non-empty |
A second `user:` line inside a commit overrides username/timestamp. Commits are blank-line separated; header-only commits (merge commits, no `-m`) give no files -> skipped. All raw lines of a commit become one multi-file commit.

### 6.2 git raw (`src/formats/gitraw.cpp`) - deprecated, read-only
First char `'c'`. Command (display only, `:32`): `git log --reverse --raw --pretty=raw`. No gen. Regexes (`:20-25`):
```
git_raw_commit    ^commit ([0-9a-z]+)
git_raw_tree      ^tree ([0-9a-z]+)
git_raw_parent    ^parent ([0-9a-z]+)
git_raw_author    ^author (.+) <([^@>]+)@?([^>]*)> (\d+) ([-+]\d+)
git_raw_committer ^committer (.+) <([^@>]+)@?([^>]*)> (\d+) ([-+]\d+)
git_raw_file      ^:[0-9]+ [0-9]+ [0-9a-z]+\.* ([0-9a-z]+)\.* ([A-Z])[ \t]+(.+)
```
Parse (`:39-93`): `commit` line; next `tree`; then 0+ `parent`; then `author` (username = group1); next line `committer` (timestamp = `atol(group4)`, committer time, tz ignored, not zero-checked); next line (blank) consumed unchecked; message lines until first empty line; then file lines until empty line: each match adds (file=group3, action=group2). Any mismatch/EOF -> false.

### 6.3 custom (`src/formats/custom.cpp`): see sec. 7.

### 6.4 SVN (`src/formats/svn.cpp`)
First char `'<'` (`:49`). Command (`:34-47`), cwd = repo dir:
`svn log -r <START>:<STOP> --xml --verbose --quiet`, START = `{<start_date>}` or `1`; STOP = `{<stop_date>}` or `HEAD`. gen condition: `<dir>/.svn` is a directory (`:67-`).
Regexes (`:29-32`): `^<\??xml`, `^<logentry`, `^</logentry>`, `(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})`.
Parse (`:150-302`): read line; if not `^<logentry`: must match `^<\??xml`, then skip lines to first `^<logentry` (else false). Accumulate lines (joined by `\n`) until a line matches `^</logentry>`; EOF first -> false. Parse with TinyXML (failure -> false). Root child `logentry` required.
| Field | Rule |
|---|---|
| date | `<date>` required; text matched by timestamp regex (fraction and `Z` ignored); fields -> `timegm` (UTC; fallback `__timegm_hack` toggling `TZ`). NULL text would crash |
| author | optional; text null/empty -> `"Unknown"`; element absent -> username stays `""` (still valid with files) |
| paths | `<paths>` absent -> true with no files (skipped). For each `<path>` (iteration uses `NextSiblingElement()` of any name): no `action` attr -> skip. `kind=="dir"`: only action `D` kept, trailing `/` appended if missing; `kind` absent/other (incl. `""`) = file. Empty text or action skipped. `addFile(text, action)` with raw SVN letter. XML entities decoded. |
Fixture `tests/logs/svn-dir-delete.log`: rev 1 adds 5 files with `kind=""`; rev 2 `<path\n kind="dir"\n action="D">/blah</path>` over several lines (OK because only `<logentry`/`</logentry>` are line-matched, and must start at column 0). Rust needs an XML parser with entity decoding (`roxmltree`/`quick-xml`).

### 6.5 Mercurial (`src/formats/hg.cpp`, `data/gource.style`)
No first-char check. Command (`:26-54`, `:71-`), cwd NOT changed:
`hg log [RANGE] -r 0:tip --style '<resource_dir>gource.style' -R "<dir>" > tmp`; RANGE = `--date '<start> to <stop>'` | `--date '>START'` | `--date '<STOP'` | empty. gen: `<dir>/.hg` directory. Windows: `'` -> `"`.
`data/gource.style` verbatim:
```
changeset = '{file_adds}{file_copies}{file_mods}{file_dels}'
file_mod  = "{date|hgdate}|{author|person}|M|{file_mod}\n"
file_add  = "{date|hgdate}|{author|person}|A|{file_add}\n"
file_del  = "{date|hgdate}|{author|person}|D|{file_del}\n"
file_copy = "{date|hgdate}|{author|person}|A|{name}\n"
```
Regex (`:24`): `^([0-9]+) -?[0-9]+\|([^|]+)\|([ADM]?)\|(.+)$` = (unix time, [tz offset ignored], username, action, filename).
Grouping (`:105-149`): loop `parseCommitEntry`; first line sets timestamp+username; later lines with equal (timestamp, username) append; a different line is pushed back via `lastline` and ends the commit; a non-matching line ends the commit and is dropped. Empty action -> `"A"`. Returns `!files.empty()`. Two commits by one user in the same second merge. Order: adds, copies (A), mods, deletes.

### 6.6 Bazaar (`src/formats/bzr.cpp`)
No first-char check. Command (`:28-38`, gen `:55-`): `bzr log --verbose -r <S>..<E> --short -n0 --forward <dir> > tmp` (cwd unchanged); S = `date:<start_date>` or `1`; E = `date:<stop_date>` or `-1`. gen: `<dir>/.bzr` directory.
Regexes (`:23-24`):
```
bzr_commit_regex  ^ *([\d.]+) (.+)\t(\d{4})-(\d+)-(\d+)(?: \{[^}]+})?(?: \[merge\])?$
bzr_file_regex    ^ *([AMDR])  (.*[^/])$
```
Parse (`:88-125`): header must match (else false, line consumed); username = group2; y/m/d -> local `mktime` at **midnight**. Then lines until blank/EOF; matching file regex adds (group2, group1); other lines (messages, dir entries ending `/`) ignored. Always true after a header (empty commits skipped by validation). `R` passes as modify action; filename is raw `old => new`.

### 6.7 CVS-EXP (`src/formats/cvs-exp.cpp`)
No first-char check. Command (display only, `:28`): `cvs-exp.pl -notree`; no gen. Regexes (`:20-26`):
```
cvsexp_commitno_regex  ^([0-9]{6}):
cvsexp_branch_regex    ^BRANCH \[(.+)\]$
cvsexp_date_regex      ^\(date: ([0-9]{4})[-/]([0-9]{2})[-/]([0-9]{2}) ([0-9]{2}):([0-9]{2}):([0-9]{2})(?: [+-][0-9]{4})?;(.+)$
cvsexp_detail_regex    author: ([^;]+);  state: ([^;]+);(.+)$      (two spaces before "state")
cvsexp_entry_regex     \| (.+),v:([0-9.]+),?
cvsexp_end_regex       ^(=+)$
```
Parse (`:38-138`): read line (if empty read another); must match commitno. Next line: if `BRANCH [..]` expect empty line then read next. Date line must match; timestamp = local `mktime` (offset ignored). Group 7 must match detail -> username = author; state `dead` -> `D` else `M`. Next line; while it matches entry: if group1 lacks `/Attic/` add (group1, action); read next (EOF -> false). Then one blank line, message lines until blank, then skip to a line matching `^=+$` or EOF. True.

### 6.8 cvs2cl (`src/formats/cvs2cl.cpp`)
First char `'<'`. Command (display only, `:32`): `cvs2cl --chrono --stdout --xml -g-q`; no gen. Regexes (`:27-30`): `^<\??xml`, `^<entry`, `^</entry>`, `(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})Z`.
Same accumulate-then-TinyXML structure as SVN, root `<entry>` (`:40-165`). Date from child `<isoDate>` (required) but converted with **local `mktime`, not timegm** (inconsistent with svn). `<author>` optional (`GetText()` unchecked; empty -> `Unknown`). Each `<file>` child (sibling iteration of any name) needs `<name>` and `<cvsstate>`; `dead` -> `D` else `M`; empty name skipped.

### 6.9 Apache combined (`src/formats/apache.cpp`)
No first-char check, no gen. One line = one commit with one file, action always `"A"`. Regexes (`:22-26`):
```
apache_entry_start   ^(?:[^ ]+ )?([^ ]+) +[^ ]+ +([^ ]+) +\[(.*?)\] +(.*)$
apache_entry_date    (\d+)/([A-Za-z]+)/(\d+):(\d+):(\d+):(\d+) ([+-])(\d+)
apache_entry_request "([^ ]+) +([^ ]+) +([^ ]+)" +([^ ]+) +([^\s+]+)(.*)
apache_entry_agent    +"([^"]+)" +"([^"]+)" +"([^"]+)"       (result unused)
apache_hostname_parts ([^.]+)(?:\.([^.]+))?... x8             (unused)
```
Parse (`:32-128`): `start` must give exactly 4 groups: username = group1 (host/IP; optional leading vhost field skipped), group3 = date inside `[..]`, group4 = remainder (group2 = authuser unused). Date must give exactly 8 groups; month by English 3-letter name, **unknown -> January**; tz offset ignored; local `mktime`. Request regex on remainder needs >= 5 groups: method, url, proto, status, bytes (`[^\s+]+`). URL: cut at last `?`; empty -> `/`; trailing `/` -> append `index.html`. Status, bytes, agent, referrer unused. Malformed -> false.

## 7. Custom log format (full spec)

`src/formats/custom.cpp`; no first-char check; no gen; also the `--output-custom-log` format.

Line: `timestamp|username|action|filename[|colour]`
Regex (`:21`): `^(?:\xEF\xBB\xBF)?([^|]+)\|([^|]*)\|([ADM]?)\|([^|]+)(?:\|#?([a-fA-F0-9]{6}))?`
| Field | Rule |
|---|---|
| BOM | optional UTF-8 BOM at start of **any** line |
| timestamp (g1) | `[^|]+`. If length > 1 and a `-` at index >= 1 -> `parseDateTime` (local unless `Z`/offset; fail -> line invalid). Otherwise `atoll`; 0 accepted only if field is exactly `"0"`. So `"abc"` rejected, `"12abc"` -> 12, `"-5"` -> -5 (negative accepted), leading whitespace skipped (`:60-68`) |
| username (g2) | `[^|]*`; empty -> `"Unknown"` |
| action (g3) | `[ADM]?`; empty -> `"A"`. Other letters (`R`, `T`) make the line fail |
| filename (g4) | `[^|]+`, cannot contain `|` (truncated at it); leading `/` added later |
| colour (g5) | optional `|`, optional `#`, exactly 6 hex digits (either case); `sscanf("%02x%02x%02x")/255.0` -> vec3 in 0..1 (not normalised). Absent -> `fileColour(filename)`. Regex has **no end anchor**: trailing text after filename/colour is ignored, `...|f.txt|zzz` silently has no colour, `...|f.txt|abcdef` colours it |
| line endings | `\r` stripped by log layer; blank/garbage lines fail the regex |

Grouping (`:41-101`): `parseCommit` loops `parseCommitEntry` until false; returns `!files.empty()`. First accepted line sets (timestamp, username); following lines with identical timestamp AND username join; first differing line is pushed back via `lastline` and ends the commit. Edge cases:
- A malformed line ends the current commit and is **dropped** (not pushed back).
- If `addFile` filters drop a file, `files` stays empty, so the next line may overwrite timestamp/username.
- Mixed actions in one (ts,user) group are fine; non-contiguous same-ts lines form separate commits. No sorting.
- `checkFormat` needs the first parse to yield >= 1 non-filtered file; a malformed first line rejects the format.
- Directory delete: filename ending `/` with `D` (`tests/logs/custom-dir-delete.log`); non-`D` directory entries ignored (`gource.cpp:1199-1204`).

### tests/logs fixtures (run via `tests/test.conf`, `stop-at-time` 5-10 s)
| File | Content | Exercises |
|---|---|---|
| `tests/logs/custom-dir-delete.log` | `1277787455|bob|A|path/to/1.txt`; three adds at `1277787457` under `path/to/blah/`; `1277787460|bob|D|path/to/` | multi-file grouping, trailing-slash dir delete |
| `tests/logs/file-removal.log` | 5 adds `path/to/file.1..5` at `...455`; deletes .1-.4 at `...456`, .5 at `...466`; trailing blank line; run with `file-idle-time=0.0` | deletes, 10 s gap |
| `tests/logs/file-to-dir.log` | `path/to/file/Not`, `.../Not/Not`, `.../Not/Not/Not`, `.../Not/Not/Not/File` | consumer behaviour (name used as file then dir), no parser edge |
| `tests/logs/svn-dir-delete.log` | SVN XML, sec. 6.4 | `kind=""`, multi-line element, dir `D` |
| `tests/logs/utf8.log` | username/path `Árvíztűrő tükörfúrógép` (valid UTF-8, spaces), ts `1277787455` A and `1277787465` D, trailing blank line | UTF-8 names with spaces |
| `tests/logs/utf8-caption.log` | `1277787455|Árvíztűrő tükörfúrógép adds a file` and `2010-06-29 16:57:45|... removes a file` — a **caption** file (`caption_regex` `^(?:\xEF\xBB\xBF)?([^|]+)\|(.+)$`, `gource.cpp:1080`), not a commit log | date-string timestamp `YYYY-MM-DD hh:mm:ss`, UTF-8 |
Other `test.conf` cases use filters (`user-filter=Andrew`, `file-filter=\.cpp`, `file-show-filter=\.cpp`) on the default path. No fixtures exist for BOM, colour field, `|` in names, CRLF or malformed lines; add them in the Rust suite.

## 8. Comparison table

| Format | 1st char | Gen from dir | Timestamp | tz | Username | Action | Grouping |
|---|---|---|---|---|---|---|---|
| git | `u` | `.git`; `git log ...` (cwd=repo) | `%ct`/`%at` epoch (0 = reject) | n/a | `%aN` | status letter | blank-line delimited |
| gitraw | `c` | no | committer epoch | ignored | author name | `[A-Z]` | header+blank lines |
| custom | none | no | epoch or parseDateTime | local unless Z/offset | field or `Unknown` | A/D/M, default A | consecutive same (ts,user) |
| svn | `<` | `.svn`; `svn log ...` (cwd=repo) | `<date>` | UTC (timegm) | `<author>`/`Unknown` | raw letter; dirs only D (+`/`) | one `<logentry>` |
| hg | none | `.hg`; `hg log ... -R dir` | hgdate epoch | offset ignored | `{author|person}` | A/M/D | consecutive same (ts,user) |
| bzr | none | `.bzr`; `bzr log ... dir` | date only | local midnight | header name | A/M/D/R | header + blank |
| cvs-exp | none | no | `(date: ..)` | local, offset ignored | author | D if dead else M | commit block |
| cvs2cl | `<` | no | `<isoDate>` | local (Z ignored) | author/`Unknown` | D if dead else M | `<entry>` |
| apache | none | no | `[dd/Mon/yyyy:hh:mm:ss +zzzz]` | local, offset ignored | host/IP | `A` | 1 line = 1 commit |
File logs and generated temp logs are seekable; stdin (`-`) is not.

## 9. Rust notes
- Line reader: split `\n`, strip one trailing `\r`; stdin reader must report "no line yet" without terminating, and drops an unterminated last line.
- Wrap regex to reproduce: unanchored search, empty subject never matches, trailing-group trimming.
- Time conversions per sec. 8 (`mktime` vs `timegm`); `libc::mktime` or `chrono::Local` for local.
- Percent seek: discard partial line after non-zero offset; keep float arithmetic if slider fidelity matters.
- XML (svn, cvs2cl): entity decoding, multi-line elements.
