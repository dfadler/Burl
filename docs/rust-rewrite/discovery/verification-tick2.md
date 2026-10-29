# Verification tick 2 (sample of 32 citations)

Checked against the source tree at the worktree HEAD. Line numbers verified by opening the cited file.

| doc | claim (short) | cite | status |
|---|---|---|---|
| app-core | SplineEdge edge_detail = min(10,(int)(ang*100.0)), floor 1 | spline.cpp:23-69, 37-39 | OK |
| app-core | FileKeyEntry shadow quad alpha*0.333, gradient colour*0.5 | key.cpp:113-152 | OK |
| app-core | interactDirs bounds +/-1, max_depth 1 unless dir_bounds.area()>10000 | gource.cpp:1418-1449 (1421-1424) | OK |
| app-core | reset creates `new RDirNode(0,"/")`; updateBounds iterates gGourceDirMap | gource.cpp:924, 1354 | OK |
| scene-model | stringHash uses XOR `gStringHashSeed^(n-i)`, signed char, negate if <0 | stringhash.cpp:32-46 | OK |
| scene-model | vec2Hash / colourHash formulas (/7, /3, %255, h==0 -> h++, normalise) | stringhash.cpp:48-57, 71-83 | OK |
| scene-model | RUser::logic order, lag rule, d<100, accel decay `max(0,1-friction*dt)` | user.cpp:231-302 | OK |
| scene-model | gStringHashSeed reset to 31 at settings load | gource_settings.cpp:490 | SHIFTED to gource_settings.cpp:489 (tests doc cites 489 correctly) |
| scene-model | RDirNode ctor shadows `parent` (`parent = 0; setParent(parent)`), NOTE at :43 | dirnode.cpp:38-39, 43 | OK |
| scene-model | user.cpp constants beam 100 / action 50 / personal space 100 | user.cpp:20-22 | OK |
| settings-cli | `--stop-on-idle` parsed, no-op | gource_settings.cpp:1376-1377 | OK (NOTE at 1376, `if` at 1377) |
| settings-cli | help text says user-friction default 0.67, code stores 1.0 and divides input (1/x) | gource_settings.cpp:162, 472, 1411-1421 | OK |
| settings-cli | file-idle-time uses atoi; invalid if <0 or (==0 and first char != '0') | gource_settings.cpp:1222 | OK |
| settings-cli | padding invalid if <=0 or >=2; default 1.1; max-file-lag only ==0 invalid | gource_settings.cpp:1466, 431, 1400 | OK |
| settings-cli | defaults and entry lines for elasticity, bloom-intensity 0.75, filename-time, hash-seed, dir-name-position range | gource_settings.cpp:974, 952, 941, 1062, 1618 | OK |
| settings-cli | `--user-idle-time` not in arg_types (conf only) | gource_settings.cpp:1248 | OK (only appears at 1248/1250, no arg_types entry) |
| log-formats | `--log-format cvs-exp` accepted by settings but logmill matches only "cvs" | logmill.cpp:238; gource_settings.cpp:779, 784 | OK |
| log-formats | auto-detect order git, hg, bzr, git-raw, cvs-exp, svn, cvs2cl, custom, apache | logmill.cpp:272-334 | OK |
| log-formats | first-char checks git 'u', gitraw 'c', svn '<', cvs2cl '<' | git.cpp:130, gitraw.cpp:34, svn.cpp:49, cvs2cl.cpp:37 | OK |
| log-formats | start_timestamp handling | logmill.cpp:86 | SHIFTED to logmill.cpp:85 |
| log-formats | queue max 100; stop_timestamp compare; help log-format line | gource.cpp:919, 1137, 1148; gource_settings.cpp:99 | OK |
| log-formats | colourHash `:73-83` | stringhash.cpp | SHIFTED: function starts at 71 |
| log-formats | all verbatim regexes (git raw x6, svn x4, cvs2cl x4, cvs-exp x6, bzr x2, hg, custom, apache x5, git_version, caption) | formats/*.cpp, gource.cpp:1080 | OK, character-for-character (doc strips C++ string escaping consistently; line ranges :20-26, :29-32, :27-30, :22-26, :23-24, :21, :36 all correct) |
| log-formats | regex.cpp: match at 149, empty subject returns -1 at 174, trailing groups trimmed / middle "" at 195-208 | regex.cpp:149, 174, 195-208 | OK |
| core-lib | `ui/`, `tga.cpp`, `timer.cpp` not in Makefile.am; tga.cpp only in gource.pro:84 | Makefile.am:11-32, gource.pro:84 | OK |
| core-lib | SDLAPP_RESOURCE_DIR=$(pkgdatadir), SDLAPP_FONT_DIR | Makefile.am:67, 85; sdlapp.cpp:215-244 | OK |
| core-lib | FXFont shadowStrength 0.7, offset (1,1), alignTop true, alignRight false | fxfont.cpp:449-453 | OK |
| core-lib | quadtree leaf rule `depth>=max || items<max_items`; min_delta_msec 8; FT_Set_Char_Size | quadtree.cpp:38; sdlapp.cpp:317; fxfont.cpp:241-243 (call at 243) | OK |
| output-pipeline | frame-rate tick calc: 60->skip0, 30->skip1, 25->75Hz skip2 | gource.cpp:505-522 | OK |
| output-pipeline | exporter forces `scaled_dt = max_tick_rate`; stop_at_time check | gource.cpp:246-261 | OK |
| output-pipeline | PPM header `P6\n%d %d 255\n` written every frame; framerates 25/30/60 only | ppm.cpp:175, 186; core/settings.cpp:456-470 | OK |
| output-pipeline | exporter forces stop_at_end unless dont_stop/loop/"-"; multi-repo 60 s default | gource_shell.cpp:219-226, 228-241 | OK |
| output-pipeline | start camera z = -300 | gource.cpp:138 | SHIFTED to gource.cpp:140 |
| output-pipeline | screenshot naming gource-%04d.png, overwrites 9999 when full | gource.cpp:2234-2255 | OK |
| deps | FreeType `freetype2 >= 9.0.3` | configure.ac:39 | SHIFTED to configure.ac:36 |
| deps | SDL2_image must export IMG_LoadPNG_RW / IMG_LoadJPG_RW | configure.ac:48-49 | SHIFTED to configure.ac:45-46 |
| deps | PCRE2 40, GLEW 42, SDL2 43, libpng 44, GLM 63, TinyXML 70-76, GLU 32 | configure.ac | SHIFTED: actual 37, 38, 39, 40, 65, 67-74, 31 |
| deps | gource.pro tga.cpp :84, tinyxml :61-64, boost_system :26, c++11 :10, ASSERTS :15; Makefile.am tinyxml conditional :57-63, man hook :74-79, fonts :81-90 | gource.pro; Makefile.am | OK |
| deps | AC_CHECK_FUNCS timegm/unsetenv at :17; HAVE_TIMEGM uses in svn.cpp:113,138,238 | configure.ac:17; svn.cpp | OK |
| tests | Makefile.am test rules :92-103; Boost >= 1.69 at configure.ac:48-53 | Makefile.am, configure.ac:49 | OK |
| tests | putenv TZ at datetime_tests.cpp:27 and :31; `+13` case duplicated at l.101/104 | datetime_tests.cpp | OK (duplicate present) |
| tests | unseeded rand() sites dirnode.cpp:625, user.cpp:106,128, gource.cpp:1555; srand(time) gource_settings.cpp:1319-1320 | as cited | OK |
| tests | writeCustomLog then exit(0) | main.cpp:119-122 | OK |
| tests | hash seed default 31 at gource_settings.cpp:489, `--hash-seed` 1062-1066 | as cited | OK |

## Systemic issues

Counts: 32 core samples plus extras; about 30 OK, 8 SHIFTED, 0 WRONG on substance. All simulation and parse claims (formulas, constants, defaults, ordering, quirk claims, every regex) hold. The only drift is line numbers. deps-and-rust-mapping.md is consistently off for configure.ac, where PKG_CHECK, GLM and TinyXML cites sit 2 to 4 lines from the real positions (the top of the file, :6 :17 :19-26, and Boost :52-53 are right). Other docs are off by 1 to 2 lines only occasionally (scene-model :490 vs 489, log-formats logmill :86 vs 85, output-pipeline gource.cpp:138 vs 140). Cross-doc inconsistency: scene-model.md cites the hash-seed reset at :490 while tests-and-golden-strategy.md has the correct :489. Not verified: external crate versions and licensing claims in deps (network facts), Windows/macOS-only code paths.
