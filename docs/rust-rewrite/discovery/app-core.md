# Application core and main loop (discovery)

Scope: src/gource.cpp/.h, gource_shell, main, logmill, key, slider, textbox, caption, zoomcamera, bloom, spline.
Part 3 (this head) covers the smaller files; Part 1 and Part 2 below cover gource.h/gource.cpp (merged verbatim from sub-agent output).
Note: the outer loop (SDLApp::run, dt computation, fps, `min_delta_msec` enforcement) lives in src/core/sdlapp.cpp, outside this subtask; GourceShell only sets `min_delta_msec = 16` (src/gource_shell.cpp:29) and consumes `update(t, dt)`, `fps`, `appFinished`.

# Part 3: smaller files

## src/main.cpp (238 lines)

| Step | Behaviour | Line |
|---|---|---|
| Init | `SDLAppInit("Gource","gource",exepath)`; exepath=argv[0] on non-Windows | 22-29 |
| Parse args | `gGourceSettings.parseArgs(argc,argv,conf,&files)` into a ConfFile plus positional `files` | 43 |
| Implicit config file | if no `--load-config` and files non-empty: first file ending `.conf`/`.cfg`/`.ini` (length > ext len) that loads as ConfFile becomes `load_config` and is erased from files. `file.rfind(".conf") == (file_length-5)` compares size_t to int; npos never equals a valid length so non-matching is safe | 45-72 |
| Log level | `Logger::getDefault()->setLevel(log_level)` | 75 |
| Load config | `conf.clear(); conf.load(load_config); parseArgs(argc,argv,conf)` (CLI overrides file) | 85-91 |
| Path | LAST positional file becomes `path` in every `[gource]` section (or creates section "gource") | 94-106 |
| Import | `importDisplaySettings(conf)`, `importGourceSettings(conf)` (validation pass) | 109-110 |
| `--save-config` | `conf.save(file); exit(0)` | 113-116 |
| `--output-custom-log` | needs non-empty path: `Gource::writeCustomLog(path, file); exit(0)` | 119-123 |
| ConfFileException | `SDLAppQuit(exception.what())` | 125-128 |
| Display flags | frameless, multisample(4) if `multisample`, alpha if `transparent`, vsync, resizable only when `output_ppm_filename` empty | 131-149 |
| High DPI | default true; non-Windows: false if `viewport_specified && !high_dpi` | 153-163 |
| display.init | `(width,height,fullscreen,screen)`; SDL2: set window position if windowed and `window_x>=0 && window_y>=0`; hint MINIMIZE_ON_FOCUS_LOSS=0. SDLInitException -> "SDL initialization failed - %s" | 165-183 |
| Exporter | `PPMExporter(output_ppm_filename)` when set; PPMExporterException -> "could not write to '%s'" | 188-200 |
| GL | `GL_MULTISAMPLE_ARB` if multisampling | 203-205 |
| Run | `new GourceShell(&conf, exporter)`; `run()`. ResourceException -> "failed to load resource '%s'"; SDLAppException: `showHelp()` -> `gGourceSettings.help()` else SDLAppQuit(what) | 207-227 |
| Exit | delete shell/exporter, `display.quit()`, return 0 | 229-237 |

Exit paths: exit(0) after save-config / custom log; SDLAppQuit(msg) (core, not read here); normal return 0.

## src/gource_shell.cpp/.h (314/65 lines): GourceShell : SDLApp

Fields (gource_shell.h:27-36): `Gource* gource`; `bool next`; `TextureResource* transition_texture`; `float transition_interval`; `float toggle_delay`; `FrameExporter* exporter`; `ConfFile* conf`; `ConfSectionList::iterator gource_settings`. Global `gGourceShell` (gource_shell.cpp:20).

Constructor (24-45): `min_delta_msec=16`; `next=false`; `gource_settings = conf->getSections("gource")->begin()`; `gGourceSettings.repo_count = conf->countSection("gource")`; toggle_delay=0; transition_interval=0; transition_texture = display-sized RGBA texture if NPOT supported (`GLEW_ARB_texture_non_power_of_two || GLEW_VERSION_2_0`).

Multi-repo state machine (`getNext`, 205-251):
1. If a Gource exists: `transition_interval=1.0f` (1 s cross-fade), delete it (207-211).
2. Return 0 (caller sets `appFinished=true`) if `gGourceSettings.shutdown` or settings iterator at end (213-216).
3. `importGourceSettings(*conf, *gource_settings)` per section (218).
4. If exporter and not (`dont_stop || loop || path=="-"`): `stop_at_end=true` (224).
5. If `repo_count>1` and `stop_at_time<=0 && stop_position<=0`: `stop_at_time = 60.0f` (231-233).
6. Advance iterator; at end, if `repo_count>1 && exporter==0` wrap to begin (loops forever); with exporter, ends (236-243).
7. `gource = new Gource(exporter); next=false` (246-248).

`update(t,dt)` (283-314): if no gource or `isFinished()` -> `getNext()` or `appFinished=true`, return (nothing drawn that frame). Else `gource->fps=this->fps; gource->update(t,dt)`; `toggle_delay -= dt` when >0; if (`next || gource->isFinished()`) and transition texture exists: `glCopyTexImage2D` snapshot, else `blendLastFrame(dt)` (fullscreen quad of previous scene, alpha=`transition_interval`, decremented by dt, 253-281). If `next`: delete gource, `transition_interval=1.0`, `next=false`.

Input (129-184): KEYDOWN non-repeat only: ESC -> `quit()`; if `disable_input` return (after ESC); F5 -> `reload()` (shaders `reload(true)`); F11 -> `toggleWindowFrame()` (SDL2 only; blocked if `toggle_delay>0`, fullscreen, or exporter; then toggle_delay=0.25f, 72-94); RETURN -> Alt held: `toggleFullscreen()` (blocked with exporter), else if `repo_count>1` `next=true`. Then always forwards event to `gource->keyPress`. Mouse move/click/wheel forwarded (186-198). `quit()` calls `gource->quit()` and sets `gGourceSettings.shutdown=true` (200-203). `resize()`/toggles unload then reload texture/shader/font managers and gource GL resources.

## src/logmill.cpp/.h (337/71 lines): RLogMill

Fields (logmill.h:42-51): `SDL_Thread* thread; SDL_mutex* mutex; SDL_cond* cond` (mutex/cond unused); `int logmill_thread_state`; `std::string logfile, error`; `RCommitLog* clog`. States: STARTUP=0, FETCHING=1, SUCCESS=2, FAILURE=3 (logmill.h:34-39). `isFinished()` = state > FETCHING (141). State is a plain int written by worker, read by main (no atomics).

Constructor spawns SDL thread "logmill" immediately (logmill.cpp:52). `getLog()` joins thread and returns clog (153-161); `abort()` only joins, no cancellation (130-138); destructor aborts and deletes clog.

`run()` (65-128): state=FETCHING; on POSIX unblock SIGINT in thread; `log_format = gGourceSettings.log_format`; `clog = fetchLog(log_format)`; if clog and `start_timestamp != 0` (86): read commits until `commit.timestamp >= start_timestamp`, then `bufferCommit(commit)` and stop (loop also ends on `shutdown` or `isFinished()`). Catch `SeekLogException` -> error "unable to read log file" (100); `SDLAppException` -> its what() (102). If no clog and no error (105-125): logfile is directory -> (format given: "failed to generate log file for the specified time period" if start or stop timestamp set else "failed to generate log file"; Windows: `default_path` with ./gource.exe present -> empty error to trigger help; else "directory not supported"); non-directory -> "unsupported log format (you may need to regenerate your log file)". Final state SUCCESS if clog else FAILURE (127).

`findRepository(dir, fmt)` (163-187): `canonical(dir)`, walk up parents; first dir containing `.git` (dir or file), `.hg`, `.bzr`, `.svn` (checked in that order within one dir; later checks overwrite earlier, 171-174) sets format; nearest ancestor wins. Stops at root.

`fetchLog` (190-337): if format empty and logfile != "-" and is a directory: findRepository, replace logfile with repo root (filesystem_error swallowed, 208). If format given (213-269): git -> GitCommitLog then GitRawCommitLog; hg MercurialLog; bzr BazaarLog; cvs CVSEXPCommitLog; custom CustomLog; apache ApacheCombinedLog; svn SVNCommitLog; cvs2cl CVS2CLCommitLog; each gated by `checkFormat()`; no match -> 0. Auto-detect ORDER (273-332): git, mercurial, bzr, git raw, cvs-exp, svn, cvs2cl, custom, apache. Rewrite must replicate this order (first checkFormat success wins).

## src/key.cpp/.h: FileKey and FileKeyEntry (file extension legend)

FileKeyEntry fields (key.h:31-47), defaults (key.cpp:7-42): pos_y=-1, shadow=(3,3), width=90*font_scale, height=scaled_font_size+4, left_margin=scaled_font_size+4, count=0, brightness=1, alpha=0, move_elapsed=1, src_y=dest_y=-1, show=true. `display_ext` is truncated one char at a time while `font.getWidth > width-15*font_scale`, then "..." appended (34-41; measures with ctor param `font`, not member). Methods: getColour, getExt, setDestY (no-op if equal; src_y=pos_y; move_elapsed=0), colourize (`ext.empty()? white : colourHash(ext)`, 61), inc/dec/getCount/setCount, isFinished (`count<=0 && alpha<=0`, 57), logic(dt), draw. `isNew()` declared (key.h:66) but never defined.
logic (90-111): alpha fades out at 1/s when count<=0 or !show, in at 1/s to 1; slide: if pos_y != dest_y: pos_y<0 -> pos_y=dest_y, else move_elapsed+=dt, linear lerp over 1 s; `pos=(alpha*left_margin,pos_y)`.
Draw (113-152): shadow quad alpha*0.333 offset (3,3); gradient quad colour*0.5 -> colour; label at (int)pos.x+2,(int)pos.y+3; count at pos.x+width+4.

FileKey (key.h:74-99): `vector<FileKeyEntry*> active_keys`; `std::map<string,FileKeyEntry*> keymap` (ordered by ext); `update_interval`; `interval_remaining=1.0` initially (165); `show=true`; font `fontmanager.grab(font_file, scaled_font_size)`. `inc(RFile*)` creates entry keyed by `file->ext` with `file->getFileColour()` (212-226); `dec` ignores unknown ext. `logic(dt)` (251-311): `interval_remaining -= dt`; when <=0: if show: rebuild active_keys from non-finished entries (map order), sort by count DESC then ext ASC (241-249), truncate to `max(0,(int)((display.height-150)/20))` (276), dest_y = k*(scaled_font_size+6) for k=1.. (283-292; only count>0 entries get setDestY but y still advances), erase+delete finished entries (295-299); then `interval_remaining=update_interval` (302). Then logic(dt) on each active entry. `setShow` and `clear` force refresh (`interval_remaining=0`). Presentation only; but file counts depend on inc/dec calls from Gource.

## src/slider.cpp/.h: PositionSlider (timeline)

Fields (slider.h:27-39): font(16*font_scale, drop shadow), bounds, percent, mouseover (x or -1), mouseover_elapsed=1, fade_time=1, alpha=0, slidercol=white, capwidth, caption. `resize()` (48-54): gap=35; bounds from (35, height-70) to (width-35, height-35). `mouseOver(pos,&pct)`: inside bounds -> mouseover_elapsed=0, mouseover=pos.x, pct=(pos.x-min.x)/(max.x-min.x) (float); else mouseover=-1. `click` = mouseOver storing into member `percent`, then output (82-93). `logic(dt)` (108-120): not hovered -> elapsed+=dt up to 1 s; alpha moves +/-dt toward 1 (while elapsed<1) or 0. `show()` resets elapsed. Draw (136-164): marker at x=min.x+(w)*percent, line width 2, black shadow offset (2,2) alpha 0.7*alpha; caption at x=clamp(mouseover - capwidth/2, 1, width-capwidth-1), y=bounds.min.y - 25*font_scale, only while hovering.

## src/textbox.cpp/.h: TextBox

Defaults (textbox.cpp:23-36): shadow (3,3), colour (0.7,0.7,0.7), alpha 1, brightness 1, `max_width_chars=1024`, visible=false. `clear()`: rect_width=0, rect_height=2. `addLine`: truncate to 1024 chars; width=font.getWidth+6; height += fontsize+4 (64-77). `setPos(pos, adjust)` (95-116): y -= rect_height; if x+w>display.width: if x-w-fontheight>0 then x-=w else x=width-w; if y<0 then y += rect_height+fontheight; if y+h>display.height then y -= rect_height. Draw (118-158): shadow quad alpha*0.333, body colour*brightness, lines at (int)x+2, y+3 stepping fontsize+4. UI only.

## src/caption.cpp/.h: RCaption

Fields (caption.h:26-37): alpha=0, elapsed=0, pos, colour=`gGourceSettings.caption_colour`, font, caption, public `time_t timestamp`. `isFinished()` = `elapsed >= caption_duration` (43). `logic(dt)` (47-51): `fade_in = min(2.0, caption_duration/3)`; `elapsed+=dt`; `alpha = min(1, min(elapsed, max(0, caption_duration - elapsed)) / fade_in)` (trapezoid fade). Draw with colour+alpha. Scheduling and positioning are in gource.cpp (Parts 1-2).

## src/zoomcamera.cpp/.h: ZoomCamera

Fields (zoomcamera.h:27-43): pos, dest, target, up, _pos, _target, lockon, speed, lockon_time, padding, min_distance, max_distance, fov, znear, zfar.
Constants: fov=90 (zoomcamera.cpp:24); znear=0.1 (26); up=(0,-1,0) (28, Y flipped); `zfar=max_distance+1` (54); padding=1.0 (33; overwritten by Gource); speed=1; lockon=false; lockon_time=0. Camera looks down -Z; `dest.z = -distance`.
`adjust(bounds, adjust_distance=true)` (96-154): dest.xy = bounds.centre(); if adjust_distance: width/height = bounds*padding; aspect = display.width/(float)display.height; if aspect<1 height/=aspect else width/=aspect; `toa = tan(fov*0.5*DEGREES_TO_RADIANS)*2.0` (=2.0 at 90deg); distance = width/toa if `crop_vertical`, height/toa if `crop_horizontal`, else max(width,height)/toa; clamp to [min_distance,max_distance]; dest.z=-distance.
`logic(dt)` (170-188): `dp=dest-pos; dpt=dp*dt*speed`; if lockon: `dpt = dpt*lockon_time + dp*(1-lockon_time)`, then lockon_time = max(0, lockon_time - dt*0.5); if length2(dpt) > length2(dp) then dpt=dp; `pos+=dpt`; `target=(pos.x,pos.y,0)`. `lockOn(true)` sets lockon_time=1 (61-68). `stop()` dest=pos. `setPos(pos,keep_angle)`. `adjustDistance()` declared (zoomcamera.h:79), never defined. `gGourceVerticalCrop/HorizontalCrop` externs unused. Window/screen dependent: camera distance depends on `display.width/height`, so aspect ratio must be kept identical.

## src/bloom.cpp/.h: bloombuf (VBO of bloom quads)

`bloom_vertex` = pos vec2, colour vec4, texcoord vec4, 24 bytes padding (64 B total, bloom.h:28-38). CPU array `data`; growth: `vertex_count > data_size` -> resize to `vertex_count*2` (bloom.cpp:81-83); `add()` pushes 4 verts: pos, pos+(dx,0), pos+dims, pos+(0,dy) (textureid ignored); `update()` uploads (glBufferData if buffer_size<vertex_count else glBufferSubData, DYNAMIC_DRAW, 91-110); `draw()` GL_QUADS with client arrays (colour offset 8, texcoord offset 24). Rendering only.

## src/spline.cpp/.h: SplineEdge

`update(pos1,col1,pos2,col2,spos)` (23-69): `mid=(pos1-pos2)*0.5`; `to=pos1-spos`; `dp=min(1, dot(normalise(to),normalise(mid)))`; `ang=acos(dp)/PI`; `edge_detail = min(10,(int)(ang*100.0))` (double multiply, int truncation), floor 1 (37-39); points i=0..edge_detail, t=i/edge_detail: `p0=pos1*t+spos*(1-t)`, `p1=spos*t+pos2*(1-t)`, `pt=p0*t+p1*(1-t)` (quadratic Bezier, pos2 at t=0, pos1 at t=1), colour lerp col1*t+col2*(1-t). Label position: `pos=dir_name_position`, `s_quota=0.5-|pos-0.5|`, `p_quota=1-s_quota`, `label_pos = pos1*(p_quota*(1-pos)) + pos2*(p_quota*pos) + spos*s_quota` (63-68). Ribbon half-width 2.5 (87, 129, 142), perp=`normalise(-dy,dx)`; shadow offset (2,2), colour (0,0,0,gGourceShadowStrength) (124-129). NaN hazard: normalise of zero vector yields NaN, `(int)NaN` is UB; port should clamp detail to 1. Label pos feeds directory-name placement.

Randomness / system time in these files: none (no rand/time calls in main, shell, logmill, key, slider, textbox, caption, zoomcamera, bloom, spline). See Parts 1-2 for gource.cpp.

---

# Part 1: gource.h and gource.cpp lines 1-1400 (sub-agent output, merged verbatim)

# gource.h (whole) + gource.cpp:1-1400 — discovery notes

Scope: gource.cpp:1365-1400 is the start of `updateUsers`; its body continues past 1400 (not covered). `logic()`, `draw*()`, `updateCamera`, `interactDirs`, `updateTime` etc. are defined after 1400 (other doc). Facts quoted from beyond 1400 are tagged (ctx). Settings defaults (`gGourceSettings.*`) live in gource_settings.cpp (not covered). All cpp refs are src/gource.cpp, all h refs src/gource.h unless prefixed.

## 1. Globals
| Name | Type | Default | Line | Notes |
|---|---|---|---|---|
| gGourceDrawBackground | bool | true | cpp:21 | |
| gGourceQuadTreeDebug | bool | false | cpp:22 | toggled by key Y (cpp:725) |
| gGourceMaxQuadTreeDepth | int | 6 | cpp:23 | user quadtree depth when spread out (cpp:1309) |
| gGourceUserInnerLoops | int | 0 | cpp:25 | accumulates `uff.getLoopCount()` (cpp:1327); debug stat |
| caption_regex | Regex | `^(?:\xEF\xBB\xBF)?([^|]+)\|(.+)$` | cpp:1080 | optional UTF-8 BOM; g1 = time (no `|`), g2 = text (non-empty) |
| gGourceForceGravity (extern) | float | 10.0 | src/dirnode.cpp:22 | keys `[` `]`; used dirnode.cpp:673 — SIM STATE |
| gGourceGravity (extern) | bool | true | src/dirnode.cpp:26 | key Z — SIM STATE |
| gGourceRemovedFiles (extern) | vector<RFile*> | empty | src/file.h:95 | cleared in reset (cpp:884) |
| gGourceDirMap (extern) | map<string,RDirNode*> | | | iterated in updateBounds (cpp:1354); erased in ~RDirNode (dirnode.cpp:92) |

## 2. class Gource : SDLApp (h:52-294); everything private until `public:` at h:263
| Field | Type | h line | Initial value / where set |
|---|---|---|---|
| logfile | std::string | 53 | gGourceSettings.path (cpp:29) |
| frameExporter | FrameExporter* | 55 | 0 (cpp:129); set in setFrameExporter (cpp:521). Ctor assigns 0 at :129 before calling setFrameExporter at :165 |
| logmill | RLogMill* | 57 | `new RLogMill(logfile)` (cpp:163), async loader |
| commitlog | RCommitLog* | 59 | 0 (cpp:30); fetched in logic (ctx cpp:1583) |
| slider | PositionSlider | 60 | init() (cpp:71); show() iff exporter==0 && repo_count==1 (cpp:168) |
| camera | ZoomCamera | 61 | `ZoomCamera(vec3(0,0,starting_z), vec3(0,0,0), camera_zoom_default, camera_zoom_max)` (cpp:148); setPadding(padding) (cpp:149) |
| file_key | FileKey | 63 | `FileKey(1.0f)` (cpp:146); cleared in reset (cpp:952) |
| debug, trace_debug | bool | 65 | false (cpp:126-127); keys Q / W |
| manual_zoom / manual_rotate / manual_camera | bool | 67-69 | false in setCameraMode (cpp:529-535) and reset (cpp:900-911) |
| rotation_remaining_angle | float | 71 | 0.0 (reset cpp:902) |
| cursor | MouseCursor | 73 | hidden if hide_mouse (cpp:120) |
| grab_mouse | bool | 75 | false (cpp:113, 913) |
| mousemoved / mouseclicked / mousedragged | bool | 76-78 | false (cpp:114-116, 915-917) |
| cursor_move | vec2 | 80 | (0,0) (cpp:906); += drag xrel,yrel (cpp:379) |
| recolour | bool | 82 | false (cpp:889); key S -> true |
| use_selection_bounds | bool | 84 | false (cpp:897) |
| selection_bounds | Bounds2D | 85 | reset() (cpp:898) |
| rotate_angle | float | 87 | 0.0 (cpp:921) |
| mousepos | vec2 | 89 | NOT initialised in ctor; read at cpp:323,331,442,494 |
| last_percent | float | 91 | 0.0 (cpp:950) |
| stop_position_reached | bool | 93 | false (cpp:107); never cleared by reset/seek |
| tag_seq, commit_seq | int | 95 | both 1 (cpp:958-959) |
| mouse_hits | GLint | 97 | uninit (render picking) |
| hoverFile, selectedFile | RFile* | 99-100 | 0 |
| hoverUser, selectedUser | RUser* | 102-103 | 0 |
| file_vbo,user_vbo,edge_vbo,action_vbo | quadbuf | 105-108 | render only |
| bloom_vbo | bloombuf | 110 | render only |
| selectionDepth | GLuint | 112 | uninit |
| root | RDirNode* | 114 | 0 (cpp:153); `new RDirNode(0,"/")` in reset (cpp:924) |
| displaydate / date_x_offset | std::string / int | 116-117 | "" / 0 (cpp:139) |
| bloomtex, beamtex, logotex, backgroundtex, usertex | TextureResource* | 119-123 | bloom = "bloom_alpha.tga" if `transparent` else "bloom.tga" (cpp:74); beam "beam.png" (cpp:77); user "user.png" clamp-edge (cpp:78); logo, background optional |
| shadow_shader, text_shader, bloom_shader | Shader* | 125-127 | 0; if !ffp: "shadow","bloom","text" (cpp:80-86) |
| font_texel_size | float | 129 | `1.0f / min(512, GL_MAX_TEXTURE_SIZE)` (cpp:91) |
| textbox | TextBox | 131 | font 18*font_scale, brightness 0.5, show() (cpp:142-144) |
| font, fontlarge, fontmedium, fontcaption, fontdirname | FXFont | 133 | see §4 |
| first_read | bool | 135 | true (cpp:111); set true on loop restart (ctx cpp:1709) |
| paused | bool | 136 | false (cpp:110) |
| reloaded | bool | 137 | false; reload() sets true (cpp:240) |
| take_screenshot | bool | 139 | false (cpp:118); F12 |
| max_tick_rate | float | 141 | `1.0/60.0` (cpp:156; double narrowed to float) |
| frameskip, framecount | int | 142-143 | 0 (cpp:158-159); framecount++ each update (cpp:284) |
| currtime, lasttime | time_t | 145-146 | 0 (cpp:955-956); sim unix time |
| runtime | float | 147 | 0.0f (cpp:157); NOT reset by reset()/seek |
| subseconds | float | 148 | 0.0 (cpp:957) |
| splash | float | 150 | -1.0 (cpp:124); 15.0 on showSplash/key C |
| idle_time | float | 152 | 0 (cpp:954); N sets = auto_skip_seconds; accumulates in logic (ctx cpp:1407/1409) |
| 17 Uint32 *_time profiling counters | Uint32 | 154-170 | uninit; SDL_GetTicks; not sim state |
| track_users | bool | 172 | set in setCameraMode (cpp:532) |
| dir_bounds, user_bounds, active_user_bounds | Bounds2D | 174-176 | reset() (cpp:877-881) |
| commitqueue_max_size | int | 178 | 100 (cpp:919) |
| starting_z | float | 179 | -300.0f (cpp:140) |
| commitqueue | std::deque<RCommit> | 181 | cleared in reset (cpp:882) |
| users | std::map<string,RUser*> | 182 | key username; SORTED iteration |
| files | std::map<string,RFile*> | 183 | key full path; sorted |
| tagusermap | std::map<int,RUser*> | 184 | key tag id; ascending |
| captions, active_captions | std::list<RCaption*> | 186-187 | |
| dirNodeTree, userTree | QuadTree* | 189-190 | 0; userTree rebuilt each interactUsers |
| message, message_timer | std::string, float | 192-193 | timer 0.0 (cpp:904); decremented by dt (ctx cpp:1575) |

Private methods (h:195-262): setMessage(const char*,...), reset, addUser, addFile, deleteUser, deleteFile, selectBackground, selectUser, selectFile, selectNextUser, loadCaptions, readLog, logReadingError (declared; no definition/use in 1-1400), processCommit, addFileAction, dateAtPosition, toggleCameraMode, updateCamera, updateUsers, updateDirs, interactUsers, interactDirs, updateBounds, updateTime, mousetrace, canSeek, seekTo, zoom, loadingScreen, drawBackground, drawScene, updateVBOs, updateAndDrawEdges, drawFileShadows/UserShadows/Actions/Files/Users/Bloom, screenshot, changeColours, grabMouse.
Public (h:263-293): ctor(FrameExporter* =0), dtor, static writeCustomLog, setCameraMode(string)/(bool), setFrameExporter, showSplash, isBusy, logic, draw, init, unload, reload, quit, update, keyPress, mouseMove, mouseClick, mouseWheel (SDL2 only, h:291).

## 3. Public methods (defined in 1-1400)
| Method | Lines | Behaviour |
|---|---|---|
| Gource() | cpp:27-169 | §4 |
| writeCustomLog (static) | cpp:171-214 | RLogMill(logfile); no log: error non-empty -> `SDLAppQuit(error)` else silent return (cpp:178-182). Output "-" = stdout else `fopen(path,"w")`, silent return on failure (cpp:191). `while(!isFinished())`: nextCommit false -> break if !seekable else continue (cpp:198-203). Per file line: `"%lld|%s|%s|%s\n"` = (long long)timestamp, username, action, filename (cpp:207) = custom log format. fclose unless stdout |
| ~Gource | cpp:216-224 | reset(); delete logmill, root; `gGourceSettings.setGourceDefaults()` |
| init / quit | cpp:226, 243 | both empty (Esc -> quit() does nothing here) |
| unload | cpp:229 | unload the 5 vbos |
| reload | cpp:239 | reloaded = true |
| update | cpp:246-285 | §5 |
| setFrameExporter | cpp:505-522 | §5 |
| showSplash | cpp:501 | splash = 15.0 |
| setCameraMode(string) | cpp:524 | `mode == "track"` |
| setCameraMode(bool) | cpp:528-538 | manual_rotate/zoom/camera=false; track_users=arg; if selectedUser!=0 `camera.lockOn(arg)`; writes `gGourceSettings.camera_mode = "track"/"overview"` |
| keyPress / mouseMove / mouseClick / mouseWheel | | §8 |

## 4. Constructor option->field mapping (cpp:27-169, execution order)
| Step | Line | Detail |
|---|---|---|
| logfile | 29 | settings.path |
| GL2 fallback | 33 | `!GLEW_VERSION_2_0` => `ffp = true` |
| file_graphic | 35-37 | if null: grab "file.png" mipmap=true, GL_CLAMP_TO_EDGE |
| font scale | 39-48 | only if `default_font_scale`: if `display.viewport_dpi_ratio.x > 1.0f` font_scale = that ratio; else `font_scale = (float)(1 + max(0, display.height / 1600))` (INTEGER division: h<1600 -> 1, 1600..3199 -> 2, 3200.. -> 3). Then `setScaledFontSizes()`. debugLog "setting font scale for resolution %d x %d to %.2f" |
| fontlarge | 50-52 | size `42*font_scale`; dropShadow; roundCoordinates(true) |
| fontmedium | 54-56 | `scaled_font_size`; shadow; round false |
| fontcaption | 58-61 | `caption_size` (not scaled in ctor); shadow; round false; alignTop(false) |
| font | 63-65 | `14*font_scale`; shadow; round true |
| fontdirname | 67-69 | `scaled_dirname_font_size`; shadow; round true |
| textures | 74-78 | per §2 |
| shaders | 82-86 | only if !ffp |
| logo | 97-100 | if `logo.size()>0`: mipmap = `!(GLEW_ARB_texture_non_power_of_two || GLEW_VERSION_2_0)`; grabFile |
| background | 103-105 | `background_image` grabFile |
| hide_mouse | 120-122 | cursor.showCursor(false) |
| camera | 148-151 | start (0,0,-300); `setCameraMode(settings.camera_mode)` ("track" -> true, anything else overview) |
| tick/time | 156-159 | max_tick_rate 1/60, runtime 0, frameskip 0, framecount 0 |
| reset() | 161 | |
| logmill | 163 | |
| exporter | 165 | `setFrameExporter(exporter, settings.output_framerate)` if non-null |
| slider | 168 | visible only if no exporter and `repo_count==1` |
No option clamping is performed in this ctor. Runtime clamps exist only in key handlers (§8) and zoom().

## 5. Timing model
update(t, dt) (cpp:246-285) — parameter `t` is ignored; runtime is kept internally:
1. `scaled_dt = min(dt, max_tick_rate)` (cpp:248), f32 seconds, cap 1/60 by default.
2. If frameExporter != 0: `scaled_dt = max_tick_rate` (fixed step) (cpp:251-253).
3. `scaled_dt *= gGourceSettings.time_scale` (cpp:256).
4. `if(!paused) runtime += scaled_dt` (cpp:259). logic()/draw() still run when paused with nonzero scaled_dt (pause handled in logic, ctx).
5. `stop_at_time > 0.0 && runtime >= stop_at_time` -> stop_position_reached = true (cpp:261) (f32 accumulation).
6. `logic(runtime, scaled_dt)` timed by SDL_GetTicks (cpp:263-267); `draw(runtime, scaled_dt)` (cpp:269).
7. If exporter && commitlog && !settings.shutdown: `framecount % (frameskip+1) == 0` -> `frameExporter->dump()` (cpp:272-276).
8. If !hide_mouse: `cursor.logic(dt); cursor.draw()` using REAL unscaled dt (cpp:278-282).
9. framecount++ (cpp:284).

setFrameExporter(exporter, fps) (cpp:505-522): framecount=0; frameskip=0; `g=fps; while(g<60){ g+=fps; frameskip++; }`; `max_tick_rate = 1.0f/(float)g`; examples: 60 -> g60, skip0, 1/60; 30 -> g60, skip1; 25 -> g75, skip2 (1/75); 24 -> g72, skip2; 120 -> g120, skip0. fps<=0 -> infinite loop (no guard).

Logic-side time (ctx, beyond 1400): message_timer and splash decrement by dt (ctx cpp:1575-1576); `time_inc = dt*86400.0*days_per_second; seconds=(int)time_inc` (ctx cpp:1724-1725, truncation); first `currtime==0 && !commitqueue.empty()` -> `currtime=lasttime=commitqueue[0].timestamp; subseconds=0; loadCaptions()` (ctx cpp:1715-1719); loop restart: `settings.loop && commitqueue.empty() && seekable && idle_time >= loop_delay_seconds` -> first_read=true; seekTo(0.0); readLog() (ctx cpp:1707-1712); auto-skip: `auto_skip_seconds>=0 && idle_time>=auto_skip_seconds && !stop_position_reached` (ctx cpp:1749). Init: waits for logmill finished; error -> `throw SDLAppException(error)` (ctx cpp:1590); no log and no error -> exporter->stop(), SDL_Quit(), `SDLAppException("")` with setShowHelp(true) (ctx cpp:1594-1601); then `start_position>0` -> seekTo(start_position) (ctx cpp:1604-1606).

## 6. Commit ingestion
readLog() (cpp:1131-1186):
- return if stop_position_reached (cpp:1132).
- loop cond (cpp:1137): `(hasBufferedCommit() || !isFinished()) && (commitqueue.empty() || (commitqueue.back().timestamp <= currtime && commitqueue.size() < commitqueue_max_size))` => always read one when empty; otherwise read ahead while newest queued commit not in the future and queue <100 (size_t vs int compare).
- `nextCommit` false -> break if !seekable else continue (cpp:1141-1146) (can spin on seekable logs).
- `stop_timestamp != 0 && commit.timestamp > stop_timestamp` -> stop_position_reached = true; break; commit discarded (cpp:1148-1151).
- push_back (cpp:1153).
- `first_read && commitqueue.empty()` -> `throw SDLAppException("no commits found")` (cpp:1156-1158); then first_read=false.
- `!isFinished() && isSeekable()` -> `last_percent = getPercent(); slider.setPercent(last_percent)` (cpp:1162-1165). Not updated after finish.
- stop: `(stop_at_end && is_finished) || (stop_position > 0.0 && seekable && (is_finished || last_percent >= stop_position))` -> stop_position_reached = true (cpp:1170-1178).
- `(is_finished || stop_position_reached) && file_idle_time_at_end > 0.0f` -> `settings.file_idle_time = file_idle_time_at_end` (cpp:1180-1182) (permanent mutation).

processCommit(commit, t) (cpp:1188-1242), iterates commit.files in list order:
- Directory entry (non-empty name ending '/'): only action "D"; others `continue` (cpp:1199-1204). "D": `root->findDirs(filename, dirs)`; for each dir `getFilesRecursive`; for each file `addFileAction(commit, cf, file, t)` (cpp:1206-1228). Order depends on dirnode.cpp.
- Else: `files.find(filename)`; if absent `addFile(cf)` REGARDLESS of action (D/M on unseen file creates it); if addFile returns 0 skip entry (cpp:1231-1237). Then addFileAction.

addFile (cpp:991-1019): return 0 if `max_files > 0 && files.size() >= max_files` (cpp:995); `file_as_dir = filename (+ "/" if not ending '/')`; `root->isDir(file_as_dir)` -> 0 (cpp:998-1001; empty filename => `[size()-1]` UB); `tagid = tag_seq++` (cpp:1003); `new RFile(filename, cf.colour, vec2(0,0), tagid)`; `files[filename]=file`; `root->addFile`; `file_key.inc(file)`; `while(root->getParent()) root = root->getParent()` (debugLog "parent changed to %s") (cpp:1013-1016).

addUser (cpp:1021-1041): pos = `dir_bounds.centre()` if `dir_bounds.area() > 0` else (0,0); `tagid = tag_seq++` (same counter as files); `new RUser(name,pos,tagid)`; users[name], tagusermap[tagid].

addFileAction (cpp:1244-1290): find user by `commit.username` else addUser; for a new user: `highlight_all_users` -> setHighlighted(true), else first non-empty `highlight_users` entry exactly equal to name -> setHighlighted(true) (cpp:1258-1269). `commit_seq++` per action (cpp:1277; unused in range). Action: "D" -> RemoveAction(user,file,commit.timestamp,t); "A" -> CreateAction(...); ANY other -> ModifyAction(user,file,timestamp,t,cf.colour) (cpp:1279-1287). `user->addAction`.
Users/files are created when processCommit runs (from logic popping the queue, ctx), not at read time. Users are created only if at least one file survived (comment cpp:1245).

deleteFile (cpp:962-988): root->removeFile; clear hoverFile; selectFile(0) if selected; for every user (map order) `fileRemoved(file)`; `files.erase(fullpath)`; `file_key.dec(file)`; delete. deleteUser (cpp:1043-1059): clear hover; selectUser(0) if selected; erase from users and tagusermap; delete.

## 7. reset() (cpp:877-960) — ctor, dtor, seekTo
camera.reset; user/active_user/dir bounds reset; commitqueue.clear; tagusermap.clear; gGourceRemovedFiles.clear; delete userTree & dirNodeTree (set 0); recolour=false; selected/hover file=0; selection bounds reset; manual_rotate/zoom=false; rotation_remaining_angle=0; message_timer=0; cursor_move=0; selectedUser/hoverUser=0 (without setSelected(false)); manual_camera=false; grab_mouse=false (SDL grab not released); mouse flags false; commitqueue_max_size=100; rotate_angle=0; `delete root; root=new RDirNode(0,"/")` (cpp:923-924); delete all users (map order) + clear; delete all files (map order) + clear (files freed after root deleted); delete all captions and active_captions + clear; last_percent=0; file_key.clear; idle_time=0; currtime=lasttime=0; subseconds=0; tag_seq=commit_seq=1.
NOT reset: runtime, paused (seekTo clears), stop_position_reached, first_read, framecount, splash, settings mutations (file_idle_time, hide_*, time_scale...), gGourceGravity/ForceGravity.
canSeek (cpp:1061-1065): `!hide_progress && commitlog!=0 && commitlog->isSeekable()`. seekTo(percent) (cpp:1067-1078): return if no log/not seekable; paused=false; reset(); commitlog->seekTo(percent). Afterwards logic re-seeds currtime from first queued commit and reloads captions.

## 8. Input mapping
Keyboard keyPress (cpp:655-875): disable_input returns (cpp:656); KEYUP ignored (cpp:658). Letters use keysym.sym; Esc/Tab/Space/+ = - [ ] , . / use sym (SDL2) or unicode (SDL1) (cpp:663-687). Esc -> quit() (empty). `commitlog==0` returns after that (cpp:692).
| Key | Line | Action |
|---|---|---|
| Esc | 688 | quit() (no-op) |
| F12 | 694 | take_screenshot = true |
| Q | 698 | debug toggle |
| W | 702 | trace_debug toggle |
| M | 706-719 | toggle cursor visibility (not while dragged/clicked/left pressed); writes `hide_mouse` |
| N | 721 | idle_time = auto_skip_seconds |
| Y | 725 | gGourceQuadTreeDebug toggle |
| T | 729 | hide_tree toggle |
| G | 733 | hide_users toggle |
| U | 737-751 | cycle: (hide_usernames && !highlight_all) -> show + highlight_all; (highlight_all && !hide_usernames) -> show, no highlight; else -> hide, no highlight |
| D | 753-768 | same cycle on hide_dirnames / highlight_dirs |
| F | 770-781 | (hide_filenames && !file_extensions) -> hide=false; (!hide_filenames && file_extensions) -> file_extensions=false, hide=true; else file_extensions=true, hide=false |
| R | 783 | hide_root toggle |
| K | 787 | show_key toggle |
| C | 791 | splash = 15.0 |
| V | 795 | toggleCameraMode |
| P | 799-803 | ffp toggle iff `GLEW_VERSION_2_0 && bloom_shader != 0` |
| Z | 805 | gGourceGravity toggle (SIM) |
| S | 809 | recolour = true |
| Tab | 813 | selectNextUser |
| Space | 817 | paused toggle |
| = or + | 821-827 | days_per_second: if >=1.0 `min(30.0, floorf(d)+1)` else `min(1.0, d*2)` |
| - | 829-835 | if >1.0 `max(0, floorf(d)-1)` else `max(0, d*0.5)` (exactly 1.0 -> 0.5, but + from 1.0 -> 2.0: asymmetric) |
| KP- / KP+ | 837 / 841 | zoom(true) / zoom(false) |
| [ | 845 | gGourceForceGravity /= 1.1 (SIM) |
| ] | 849 | gGourceForceGravity *= 1.1 (SIM) |
| . | 853-859 | time_scale: >=1.0 -> `min(4.0, floorf(t)+1)` else `min(1.0, t*2)` |
| , | 862-869 | >1.0 -> `max(0, floorf(t)-1)` else `max(0.25, t*0.5)`. From 1.5 gives floorf(1.5)-1 = 0.0 (can hit 0 despite 0.25 floor elsewhere) |
| / | 871 | time_scale = 1.0 |

Mouse:
| Event | Line | Behaviour |
|---|---|---|
| mouseMove | 338-398 | return if no commitlog / disable_input / hide_mouse. If grab (SDL1) warp to (w/2,h/2) and swallow the synthetic event (cpp:346-354). If dragged or RMB held: `mag=(xrel,yrel)`; RMB: manual_rotate=true; dominant axis `rotate_angle = min(1,|d|/10) * 5 * DEGREES_TO_RADIANS` (0.017453292, src/core/pi.h:33), sign of d; return (cpp:366-377). else `cursor_move += mag` (cpp:379). If grab_mouse return. Else mousepos=(x,y), mousemoved=true, cursor.updatePos; if `!hide_progress && slider.mouseOver(mousepos,&pos)` slider.setCaption(dateAtPosition(pos)) (cpp:393-396) |
| mouseWheel (SDL2) | 422-432 | y>0 zoom(true); y<0 zoom(false); only disable_input checked |
| mouseClick | 436-499 | return if no log / disable_input / hide_mouse. cursor.updatePos(mousepos). UP: LMB -> mousedragged=false; LMB or RMB and no button still down -> grabMouse(false). Only DOWN continues. SDL1 wheel buttons zoom. MMB -> toggleCameraMode. RMB -> grabMouse(true). LMB -> mouseclicked=true; if canSeek() && `slider.click(mousepos,&position)` -> seekTo(position) |
| selectBackground | 545-557 | if LMB pressed: selectUser(0); manual_camera=true; mousedragged=true; grabMouse(true) |
| grabMouse | 307-336 | SDL2 relative mode + SDL_SetWindowGrab; on release warp to mousepos; cursor.showCursor(!grab) |
| zoom(in) | 400-419 | manual_zoom=true; `zoom_multi = 1.1` (double); `distance = -camera.getDest().z`; in: `/=1.1` clamp >= camera_zoom_min; out: `*=1.1` clamp <= camera_zoom_max; `camera.setDistance(distance)` |
Selection: selectUser (cpp:560-587): no-op if same user; deselect file+user; user==0 -> `camera.lockOn(false)`; else setSelected(true) and `if(track_users) camera.lockOn(true)`. selectFile (cpp:590-615): deselects user (no camera change) and file. selectNextUser (cpp:618-653): iterate tagusermap ascending; first with `!isInactive() && tagID > currTagId && alpha >= 1.0`; else first with `!isInactive() && alpha >= 1.0`; selectUser(result, 0 deselects).

## 9. Camera constants/behaviour in range
Start pos (0,0,-300) (cpp:140,148), look-at (0,0,0); zoom multiplier 1.1 (cpp:404); rotate per mouse event max 5 deg scaled by min(1,|d|/10) (cpp:369-375); setCameraMode clears manual flags (cpp:529-535); track vs overview from `camera_mode` string; lockOn on selection when tracking. In updateUsers: user fading while selected -> selectUser(0) (cpp:1377); follow_users: for each non-idle user, if nothing selected (user and file) and a non-empty follow name == user name -> selectUser(u) (cpp:1388-1397). Camera application (cursor_move, rotate_angle, bounds fit) is in updateCamera (beyond 1400).

## 10. Bounds / quadtree (SIM)
- updateBounds (cpp:1335-1362): reset user/active bounds; over `users` (sorted): updateQuadItemBounds; user_bounds.update; active_user_bounds.update if `!isIdle()`. dir_bounds reset; over `gGourceDirMap` (sorted by abspath) only `isVisible()` nodes.
- interactUsers (cpp:1292-1333): bounds = user_bounds +/- (1.0,1.0); delete/rebuild userTree; `max_depth=1`, `gGourceMaxQuadTreeDepth` (6) iff `dir_bounds.area() > 10000.0`; `QuadTree(bounds, max_depth, 1)`; add users (map order); for each user a in map order: `UserForceFunctor uff(a); userTree->visitItemsInBounds(a->quadItemBounds, uff); a->applyForceToActions()`. In-place updates so later users see earlier results: order- and float-order-dependent.
- Spawn position of new user depends on dir_bounds at creation (cpp:1025).

## 11. Captions
loadCaptions (cpp:1082-1129): no-op if caption_file empty or unopenable (cpp:1083-1087). Per line: `ConfFile::trim`; regex must match else skip (cpp:1096-1098). Timestamp: if `matches[0].size()>1 && find("-",1)!=npos` -> `SDLAppSettings::parseDateTime(str, ts)` (skip on failure) else `(time_t)atoll(str)` and skip if 0 unless string is exactly "0" (cpp:1103-1110). Text `RCommitLog::filter_utf8(matches[1])` (cpp:1112). Skip if `timestamp < currtime` (cpp:1115); skip if `timestamp < last_timestamp` (cpp:1118; last_timestamp init 0, updated only for kept lines). Push `new RCaption(caption, timestamp, fontcaption)` in file order (cpp:1124). Equal timestamps kept. Activation/fade logic beyond 1400.
dateAtPosition (cpp:288-305): `percent<1.0 && commitlog->getCommitAt(percent, commit)`; `localtime` + `strftime("%A, %d %B, %Y")` into 256-byte buffer (local TZ/locale); slider hover caption only, not sim.

## 12. Randomness / system time
| Use | Line |
|---|---|
| rand/srand/random: none in h or cpp:1-1400 | - |
| SDL_GetTicks (profiling only) | cpp:263, 267, 1301, 1332 |
| localtime() on commit timestamp (slider hover) | cpp:298 |
| Real wall-clock dt drives sim when no exporter: `min(dt,1/60)` (nondeterministic run to run); exporter uses fixed step | cpp:248-253 |
| cursor.logic uses real dt | cpp:280 |

## 13. Error messages / exit paths
| Condition | Effect | Line |
|---|---|---|
| writeCustomLog: log null + error | `SDLAppQuit(error)` | cpp:180 |
| writeCustomLog: log null, no error | silent return | cpp:182 |
| writeCustomLog: fopen fail | silent return | cpp:191 |
| readLog: no commits at first read | `throw SDLAppException("no commits found")` | cpp:1157 |
| logic init (ctx): logmill error | `throw SDLAppException(error)` | cpp:1590 |
| logic init (ctx): no log, no error | exporter stop, SDL_Quit, `SDLAppException("")` showHelp | cpp:1594-1601 |
| Esc | quit() no-op | cpp:688, 243 |
| bad/missing caption file or line | silently ignored | cpp:1087, 1098, 1106, 1109 |
| setFrameExporter fps<=0 | infinite loop | cpp:514 |
| debugLog strings | "setting font scale for resolution %d x %d to %.2f" (cpp:46); "parent changed to %s" (cpp:1014) | |

## 14. Rust-port hazards (sim state)
1. users/files/tagusermap/gGourceDirMap are std::map: use BTreeMap (byte-wise string order); interactUsers force application is in-place and order dependent.
2. f32 vs f64: runtime, dt, scaled_dt, max_tick_rate f32 (`1.0/60.0` narrowed from f64; `1.0f/(float)g` f32); `time_inc` f64 then `(int)` truncation (ctx); zoom multiplier 1.1 and gravity `/=1.1` are double ops on f32 values; time_scale/days_per_second use floorf.
3. tag_seq shared by files and users; incremented only on successful addFile and each addUser.
4. Unknown actions -> Modify; unseen file auto-created for any action; directory entries only honour "D".
5. addFile root-promotion loop; commit_seq per action.
6. readLog read-ahead rule and stop_timestamp discard define consumption; stop_position_reached sticky; file_idle_time setting mutated at end.
7. time_scale can reach 0 via ',' from 1.5; days_per_second can reach 0.
8. Font scale uses integer division (render only). mousepos uninitialised at ctor.

---

# Part 2: gource.cpp lines 1400-2776 (sub-agent output, merged verbatim)

# src/gource.cpp lines 1400-2776 (tail of Gource::updateUsers, then interactDirs ... draw)

Scope note: keyboard/mouse event handlers (keyPress, mouseMove, mouseClick, ~lines 300-560) are OUTSIDE this range. Within this range only the polled arrow keys (1618-1652) and mouse hover/click consumption (mousetrace, 1880-1989) appear. Main-loop dt/t production (SDLApp) is also outside; `logic(float t,float dt)` receives them. gource_a.md should cover the rest.

## 1. Functions in range

| Function | Lines | Purpose / key logic |
|---|---|---|
| tail of `updateUsers` | 1401-1416 | `users.empty() && stop_position_reached` -> `appFinished=true` (1401-1403). If `idle_users==users.size()` -> `idle_time += dt` else `idle_time=0` (1406-1410). Then `deleteUser()` on each collected inactive user (1413-1415). With 0 users idle_users(0)==size(0), so idle_time accrues. |
| `interactDirs` | 1418-1449 | Rebuilds dir quadtree every call (every frame, also while paused). Bounds = dir_bounds expanded by (1,1) each side (1421-1424). max_depth=1 unless `dir_bounds.area() > 10000.0` then `gGourceMaxQuadTreeDepth` (=6, :23) (1430-1435). `new QuadTree(bounds,max_depth,1)` (1437). Iterates `gGourceDirMap` (std::map<string,RDirNode*>, sorted by path string) and adds each non-empty node (1440-1446). Records ms via SDL_GetTicks (1426,1448), debug only. |
| `updateDirs(dt)` | 1451-1454 | `root->applyForces(*dirNodeTree); root->logic(dt);` force/physics step for the dir tree. SIM STATE. |
| `updateTime(time_t)` | 1456-1473 | 0 -> displaydate="" and return. Else `localtime()` (LOCAL TZ) + `strftime(buf[256], date_format)`. `date_offset=(int)(fontmedium.getWidth(displaydate)*0.5)`; `date_x_offset` only updated if `abs(old-new) > 5` (anti-wobble, 1472). |
| `updateCamera(dt)` | 1475-1550 | See section 4. |
| `changeColours` | 1552-1566 | `gStringHashSeed = (rand()%10000)+1` (1555) then `colourize()` all users (map order), all files (map order), and `file_key`. libc rand() (see section 8). |
| `logic(t,dt)` | 1568-1878 | Main per-frame simulation step; section 2. |
| `mousetrace(dt)` | 1880-1989 | Mouse picking/hover/click; section 5. |
| `loadingScreen` | 1991-2023 | Shown while `commitlog==0`. "Reading Log" (or "Aborting" if `shutdown`) + dots by `int(runtime*3.0f)%4` -> "", ".", "..", "..." (2003-2016). Drawn at (w/2 - textwidth/2, h/2 - 10) white (2022). Skipped if `!gGourceDrawBackground`. |
| `drawBackground` | 2025-2060 | Clear to `background_colour`, alpha 0 if `transparent` else 1 (2028); optional background image centred at (w/2 - tw/2, h/2 - th/2), white modulate (2031-2059). Returns early if `!gGourceDrawBackground`. |
| `drawScene(dt)` | 2062-2114 | Fixed order: edges -> file shadows -> actions -> files -> user shadows -> users -> bloom (each timed with SDL_GetTicks into debug counters). |
| `updateAndDrawEdges` | 2116-2172 | `root->calcEdges()` ALWAYS (even if hide_tree; 2118), affects state. If `hide_tree` return. Ortho 2D (0,w,h,0,-1,1). Shader path: rebuild edge_vbo, shadow pass offset (2.0,2.0) px (2150), `shadow_strength` 0.5 (2147), then edges. FFP path: `drawEdgeShadows()` then `drawEdges()`. Blend SRC_ALPHA/ONE_MINUS_SRC_ALPHA, beamtex. |
| `drawActions` | 2174-2194 | Skip if hide_users. beamtex; action_vbo, or per-user `drawActions(dt)` (map order) in ffp. |
| `drawBloom` | 2196-2218 | Skip if hide_bloom. Additive blend (GL_ONE,GL_ONE) (2201). bloom_shader+bloom_vbo, or FFP `bloomtex` + `root->drawBloom(dt)`. |
| `setMessage(fmt,...)` | 2220-2232 | vsnprintf into 1024 buf (truncates); `message_timer = 5.0` s (2231). |
| `screenshot` | 2234-2254 | First free `gource-%04d.png`, pngno 1..9999, in CWD via stat() (2241-2246); if all exist, loop exits with last name tried (gource-9999.png) and overwrites it. `PNGWriter(transparent?4:3)`, `png.screenshot(filename)`; message "Wrote screenshot %s" (2253). No failure check. |
| `updateVBOs(dt)` | 2256-2296 | No-op if ffp. User VBO + action VBO rebuilt (users in map order): alpha, colour, `scaled_dims=dims`; if `fixed_user_size`: `*= (-cam.z / -starting_z)` (2273; starting_z=-300 at :140). Quad at `pos - dims*0.5`, colour (r,g,b,alpha). Bloom VBO via `root->updateBloomVBO(vbo,dt)`, file VBO via `root->updateFilesVBO(vbo,dt)`. |
| `drawFileShadows` | 2298-2322 | Skip hide_files. Shader: strength 0.5, offset (2.0,2.0) px (2314), file_graphic tex. FFP: `root->drawShadows(dt)`. |
| `drawUserShadows` | 2324-2352 | Skip hide_users. offset `(2.0,2.0)*user_scale` (2337), strength 0.5. FFP: `user->drawShadow(dt)` per user. |
| `drawFiles` | 2354-2374 | Skip hide_files. `trace_debug` disables texturing. |
| `drawUsers` | 2376-2399 | Skip hide_users. `trace_debug` disables texturing. |
| `draw(t,dt)` | 2401-2776 | Full frame; section 3. |

## 2. Gource::logic(t, dt) order (1568-1878)

| # | Lines | Step | Notes |
|---|---|---|---|
| 1 | 1570-1573 | `if(shutdown && logmill->isFinished()) {appFinished=true; return;}` | exit path |
| 2 | 1575-1576 | `message_timer -= dt` if >0; `splash -= dt` if >0 | real dt, not scaled by speed |
| 3 | 1579-1607 | If `commitlog==0`: return until `logmill->isFinished()`; take `getLog()`, `getError()`. If no log: error non-empty -> `throw SDLAppException(error)` (1590); else stop `frameExporter` (if any), `SDL_Quit()`, throw `SDLAppException("")` with `setShowHelp(true)` (1593-1600) -> usage printed. If `start_position>0.0` -> `seekTo(start_position)` (1604-1606). |
| 4 | 1609-1611 | `file_key.logic(dt); slider.logic(dt)` | |
| 5 | 1613-1652 | Poll arrow keys (SDL2 `SDL_GetKeyboardState` scancodes; SDL1 `SDL_GetKeyState`): right -> cursor_move.x=+10; left x=-10; up y=-10; down y=+10; each sets `manual_camera=true`. Assignments (not +=); order right,left,up,down so left beats right, down beats up. Runs while paused. |
| 6 | 1655-1685 | If `rotate_angle != 0`: s=sinf, c=cosf. If manual_rotate: rotate about `vec2(camera.getPos())`: `root->rotate(s,c,centre)` and each user pos `rotate_vec2(pos-centre,s,c)+centre`; else about origin (`root->rotate(s,c)`; users `rotate_vec2(pos,s,c)`). `rotate_angle=0`. SIM STATE (float sin/cos). |
| 7 | 1687-1690 | `if(recolour) {changeColours(); recolour=false;}` | uses rand() |
| 8 | 1693-1699 | If `paused`: `updateBounds(); interactUsers(); interactDirs(); updateCamera(dt); return;` | No updateUsers/updateDirs/commits/captions/time. interactUsers still runs follow-user selection and idle accounting? (idle_time += dt happens in interactUsers/updateUsers tail at 1406-1410; see which function owns lines 1365-1416 - it is `updateUsers`, so NOT run while paused). |
| 9 | 1702-1704 | If `commitqueue.empty()` -> `readLog()` | |
| 10 | 1707-1713 | Loop: if `loop && commitqueue.empty() && commitlog->isSeekable()` and `idle_time >= loop_delay_seconds` (default 3.0 gource_settings.cpp:416; <=0 rejected :734) -> `first_read=true; seekTo(0.0); readLog();` | |
| 11 | 1715-1720 | First time (`currtime==0 && queue non-empty`): `currtime=lasttime=queue[0].timestamp; subseconds=0.0; loadCaptions();` | |
| 12 | 1722-1733 | `time_inc = (dt * 86400.0 * days_per_second)` (double expr stored in float). `seconds=(int)time_inc` (trunc toward zero); `subseconds += time_inc - (float)seconds`; if `subseconds >= 1.0` {currtime += (int)subseconds; subseconds -= (int)subseconds}; then `currtime += seconds`. FLOAT/TRUNCATION SENSITIVE; reproduce exactly. |
| 13 | 1736-1740 | Delete files in `gGourceRemovedFiles` (vector order) via `deleteFile`; clear. | |
| 14 | 1744-1774 | Commit loop: `commit = commitqueue.front()` (copy). Auto-skip (1749): if `auto_skip_seconds>=0.0 && idle_time>=auto_skip_seconds && !stop_position_reached` -> `currtime=lasttime=commit.timestamp; idle_time=0.0` (default 3.0 gource_settings.cpp:409; disabled -> -1 :717; value <=0 handled :1217). `if(commit.timestamp > currtime) break;` then `processCommit(commit,t)`. Then if `no_time_travel`: `lasttime=max(lasttime,commit.ts)`; else if `lasttime > commit.timestamp` -> `currtime = commit.timestamp`, and `lasttime = commit.timestamp`. `subseconds=0.0`; `pop_front`. Commits processed in queue order (not sorted here). |
| 15 | 1776 | `slider.resize()` | |
| 16 | 1778-1785 | `caption_height=fontcaption.getMaxHeight()`; `caption_start_y = canSeek() ? slider.getBounds().min.y - 35.0f : display.height - fontmedium.getMaxHeight() - 20.0f`; if title non-empty: `min(caption_start_y, display.height - 20.0f - fontmedium.getMaxHeight())`; `floor()`. |
| 17 | 1787-1807 | If `reloaded` (display resized): re-layout `active_captions` from caption_start_y upward (y -= caption_height), x per offset rule; `reloaded=false`. |
| 18 | 1809-1846 | Activate captions: while `captions.front()->timestamp <= currtime`: find first free slot y from caption_start_y stepping `y -= caption_height`; slot taken if any active caption `getPos().y == y` (exact float equality). x: `caption_offset==0` -> `display.width/2 - fontcaption.getWidth(text)/2` (int math); `<0` -> `width + offset - textwidth`; `>0` -> offset. setPos; move from `captions` deque to `active_captions` list. Order of deque matters: head with later timestamp blocks the rest. |
| 19 | 1848-1860 | Each active caption `logic(dt)`; erase+delete when `isFinished()`. | |
| 20 | 1863-1865 | Reset debug counters gGourceUserInnerLoops/DirNodeInnerLoops/FileInnerLoops=0. | |
| 21 | 1867 | `updateBounds()` | |
| 22 | 1869-1870 | `interactUsers(); updateUsers(t,dt);` | |
| 23 | 1872-1873 | `interactDirs(); updateDirs(dt);` | |
| 24 | 1875 | `updateCamera(dt)` | |
| 25 | 1877 | `updateTime(!commitqueue.empty() ? currtime : lasttime)` | displays currtime if more commits pending, else lasttime. |

Timing model: `dt` is per-frame seconds from SDLApp (outside range; verify in sdlapp.cpp whether fixed during frame export). Sim time `currtime` advances `dt*86400*days_per_second` s per frame. `t` (runtime) passed only to processCommit/updateUsers. Pause freezes time/commits/sim/captions but camera, bounds, and quadtrees keep updating. Stop conditions: `shutdown && log finished` (1570); `users.empty() && stop_position_reached` (1401); exceptions above; loop mode otherwise.

## 3. Gource::draw(t, dt) order (2401-2776)

1. `display.mode2D()`; `drawBackground` (2403-2405). 2. `!commitlog` -> `loadingScreen(); return` (2407-2410). 3. Frustum from camera (2412). 4. If `!hide_mouse && cursor.isVisible()` -> `mousetrace(dt)`; else clear hoverUser/hoverFile mouseOver (2416-2427). 5. GL_PROJECTION identity; `camera.focus()` (2431-2434). 6. `root->checkFrustum(frustum)` (2437). 7. Read viewport/modelview/projection; `root->calcScreenPos`, each user `calcScreenPos` (map order), selectedFile if `hide_filenames` (2441-2459). 8. `updateVBOs(dt)` (2467). 9. `drawScene(dt)` (2475). 10. Text: `display.push2D()`; if !ffp `fontmanager.startBuffer()`; `fontdirname.roundCoordinates(false)`, colour `dir_colour` alpha 1 (2494-2495); `root->drawNames(fontdirname)`; user names unless `hide_usernames||hide_users`, skipping selected (2499-2506); if !ffp commit buffer, `text_shader` `shadow_strength 0.7`, `texel_size=font_texel_size` (2523-2526), drawBuffer; then selected user/file names drawn on top (2536-2540); `pop2D`. 11. Debug bounds outline if `debug` (`track_users ? active_user_bounds : dir_bounds`, line width 2) (2547-2553); `gGourceQuadTreeDebug` outlines: dir tree green (0,1,0), user tree cyan (0,1,1), user items (0,1,0.5) (2555-2571). 12. `mode2D()`; logo at `(w,h) - (logo w,h) - logo_offset` (2583-2613). 13. `font.roundCoordinates(true)` (2615). 14. Splash when `splash>0`: texts "Gource", "Software Version Control Visualization", "(C) 2009 Andrew Caudwell"; logoheight `100*font_scale`, band corner y `h/2 - 40*fs`, "Gource" at y `h/2 - 30*fs`, line2 `h/2 + 10*fs`, line3 `h/2 + 30*fs`; full-width band colour (0,0.5,1, `splash*0.015`) (2617-2642). Initial splash value set elsewhere (ctor). 15. Date: `fontmedium` colour `font_colour`; unless `hide_date`, drawn at `(w/2 - date_x_offset, 20)` (2646-2650). 16. Title bottom-left (10, h-10), `alignTop(false)` (2652-2656). 17. Active captions `draw()` (2658-2662). 18. `file_key.draw(); file_key.setShow(show_key)` (2665-2666). 19. `slider.draw(dt)` if `canSeek()` (2669-2671). 20. Hover textbox: if `hoverFile && hoverFile != selectedFile`: text=file name, extra line = path with first char erased (2676-2681) if non-empty, colour file colour; else if `hoverUser && != selectedUser`: user name + colour. `setPos(mousepos,true)` (2674-2692). 21. Debug block (2696-2757). 22. `mousemoved=false; mouseclicked=false` (2759-2760); not cleared when draw returns early at 2409. 23. `take_screenshot` -> `screenshot()` (2762-2765), taken before the message overlay. 24. Message at (1,3) if `message_timer>0` (2767-2775).

Debug text lines (x=1, y=20..740 step 20): FPS, Days Per Second, Commit Queue, Users, Files, Dirs, Log Position (`commitlog->getPercent()`), Camera, Gravity (`gGourceForceGravity`), Update Tree/VBOs/Projection/Draw Scene sub-timings, Mouse Trace, Logic Time, inner loops, QTree stats, Dir Bounds Ratio, String Hash Seed (2738), VBO stats (non-ffp, 620-720), selected-file dir stats at y=740 (2754). Format strings at 2703-2756.

## 4. Camera (updateCamera 1475-1550)

| Item | Lines | Detail |
|---|---|---|
| auto_rotate | 1479 | `!manual_rotate && !disable_auto_rotate` |
| Manual camera | 1482-1501 | If `cursor_move` non-zero: `cam_rate = -cam.z / 5000.0f`; `delta = cursor_move * cam_rate * 10.0f`; pos.xy += delta; `camera.setPos(pos,true); camera.stop();` auto_rotate=false; `cursor_move = 0`. If `manual_camera` and cursor_move zero: camera not adjusted. |
| Auto camera | 1503-1520 | Bounds: if `track_users && (selectedFile||selectedUser)` -> bounds of selected user `getPos()` and selected file `getAbsolutePos()`; else if `track_users && idle_time==0` -> `active_user_bounds`; else `dir_bounds`. `camera.adjust(cambounds, !manual_zoom)` (ZoomCamera; internals outside range). |
| Step | 1522 | `camera.logic(dt)` always. |
| Auto rotation | 1525-1549 | If `rotation_remaining_angle > 0`: `angle_rate = max(dt, 1 - fabs(remaining/90 - 0.5)*2) * dt` (ease, peak mid-turn); `rotate_angle = min(remaining, 90*angle_rate)`; remaining -= rotate_angle; `*= DEGREES_TO_RADIANS`. Applied in next frame's logic step 6. Else if `!cursor.rightButtonPressed() && dir_bounds.area() > 10000.0f`: `aspect = w/(float)h`; `ratio = aspect>1 ? bw/bh : bh/bw`; if ratio < 0.67 -> `rotation_remaining_angle = 90`. If !auto_rotate: remaining=0 (1547-1549). No zero-divisor guard. |
| Initial | gource.cpp:140,148 | starting_z=-300.0; `ZoomCamera(vec3(0,0,starting_z), origin, camera_zoom_default, camera_zoom_max)`. |

## 5. Selection / hover (mousetrace 1880-1989)

- Unproject: `projected = vec2( -(mx*2 - W)/H , 1 - 2*my/H ) * cam.z; += cam.xy` (1884-1888); uses H for both axes. Float math.
- Users (unless hide_users): `userTree->getItemsAt(std::set<QuadItem*>, pos)`; set ordered by POINTER ADDRESS (nondeterministic); first user with `!isFading()` and `quadItemBounds.contains(pos)` wins (1895-1909).
- Files (only if no user hit and !hide_files): `dirNodeTree->getItemsAt(set<QuadItem*>)` (pointer-ordered); for each dir iterate `getFiles()` list; first `!isHidden() && overlaps(pos)` wins within that dir via break, but the outer loop has no break, so a later dir in pointer order overwrites fileSelection (1917-1933).
- Hover application: file branch checked first (1936) but since file search only runs when no user hit, a user hit wins. File hover clears hoverUser and vice versa; neither -> both cleared (1968-1973).
- Click (`mouseclicked`): hoverUser -> `camera.lockOn(false); selectUser`; hoverFile -> `camera.lockOn(false); selectFile`; else `selectBackground()` (1975-1986). Those functions are outside range (selectBackground at gource.cpp:545).
- Follow-user (tail of interactUsers, 1385-1398): only if nothing selected and user not idle and name equals a non-empty `follow_users` entry; every match calls selectUser (last wins).

## 6. Keyboard / mouse mapping in range

Only arrow keys polled in logic (1634-1652, magnitude 10.0). `hide_mouse` / `cursor.isVisible()` gate picking (2416). `cursor.rightButtonPressed()` suppresses auto-rotate start (1537). `mousepos` used at 1884 and 2683. All other bindings are in keyPress/mouse handlers (outside range).

## 7. Caption / timeline / slider

- Captions: deque `captions` filled by `loadCaptions()` (gource.cpp:1082, outside range) at first commit time and after seeks; activated when `timestamp <= currtime` (1812). Slot allocation: stack upward from `caption_start_y` by `caption_height`, exact-equality y test (1822). `caption_offset` int: 0 centre, <0 right-aligned from right edge, >0 left px offset. Lifetime in RCaption::logic(dt) (outside).
- Slider: `slider.logic(dt)` (1611), `slider.resize()` each unpaused frame (1776), `slider.draw(dt)` only if `canSeek()` (2669; canSeek at :1061). caption_start_y uses `slider.getBounds().min.y - 35.0f` (1779).
- Date text: `updateTime`; format `gGourceSettings.date_format`; drawn at y=20 (2649).

## 8. Magic numbers (path:line, all gource.cpp)

| Value | Line | Meaning |
|---|---|---|
| (1.0f,1.0f) | 1423-1424 | quadtree bounds padding |
| 10000.0 | 1433, 1537 | dir_bounds.area threshold (deep quadtree / auto-rotate) |
| 1, 1 | 1430, 1437 | initial max_depth, 3rd QuadTree ctor arg |
| 6 | :23 | gGourceMaxQuadTreeDepth |
| 256 | 1464, 1467 | strftime buffer |
| 0.5, 5 | 1471-1472 | date offset half-width, wobble threshold px |
| 5000.0f, 10.0f | 1486, 1490 | manual camera pan rate |
| 90.0f, 0.5, 2.0, 1.0 | 1530-1544 | rotation ease/angle |
| 0.67f | 1543 | ratio threshold to start rotation |
| 10000, +1 | 1555 | hash seed range 1..10000 |
| 10.0 | 1635-1650 | arrow-key cursor_move |
| 86400.0 | 1723 | seconds/day |
| 1.0 | 1728 | subsecond carry |
| 35.0f, 20.0f | 1779, 1782 | caption layout offsets |
| 5.0 | 2231 | message timer seconds |
| 1024 | 2222, 2226 | message buffer |
| 256, 10000 | 2237, 2241 | screenshot name buf; max index |
| (2.0,2.0) | 2150, 2314, 2337 (x user_scale) | shadow offsets |
| 0.5 | 2147, 2308, 2334 | shadow_strength |
| 0.7 | 2524 | text shadow_strength |
| 3.0f, 4, 10 | 2003, 2022 | loading dot speed/count; text y offset |
| 100, 30, 40, 10, 30 (x font_scale), 0.015f, (0,0.5,1) | 2619-2641 | splash layout/colour |
| 20; 10,10 | 2649; 2654 | date y; title x,y |
| 1,3 | 2774 | message position |
| 20..740 step 20 | 2703-2754 | debug text rows |

Randomness: only `rand()` in changeColours (1555), reached via `recolour` flag (set by key handler outside range). No srand in gource.cpp (grep); seeding, if any, is elsewhere (main.cpp/settings, verify). gStringHashSeed default 31 (gource_settings.cpp:489), settable via CLI (:1066); hash seed drives colourize results (SIM-visible colours).
System time: `SDL_GetTicks()` only for debug timers (1426,1448,2066-2112,2414,2439,2465,2473,2485-2521); no sim effect. `localtime()` (1465) for date display (TZ dependent). `runtime` (loadingScreen dots) comes from the app.

## 9. Error messages / exit paths

| Path | Lines | Behaviour |
|---|---|---|
| shutdown + log finished | 1570-1573 | appFinished=true |
| log load error | 1589-1590 | throw SDLAppException(error) (message from logmill) |
| no commits / no log, no error | 1592-1600 | stop frameExporter, `SDL_Quit()`, throw SDLAppException("") with showHelp |
| no users & stop_position_reached | 1401-1403 | appFinished |
| screenshot | 2253 | message "Wrote screenshot %s"; no failure detection |
| status text | 2018 | "Reading Log" / "Aborting" |

## 10. Sim-state-affecting flags (for Rust port)

1. Time advance float/int truncation (1723-1733): time_inc double->float, subseconds float, (int) truncation.
2. Time-travel handling (1758-1769) and auto-skip placement inside the commit loop (1749).
3. Pause path runs updateBounds/interactUsers/interactDirs/updateCamera but not updateUsers/updateDirs (idle_time frozen).
4. Per-frame order: removed files deleted (1736) -> commits -> captions -> updateBounds -> interactUsers -> updateUsers -> interactDirs -> updateDirs -> updateCamera -> updateTime. Rotation (from previous frame's updateCamera) applied at the start (1655) before all of these.
5. Rotation uses sinf/cosf float; rotates dir tree and user positions.
6. Containers: users/files `std::map<string,..>` (name-sorted), `gGourceDirMap` std::map (path-sorted; quadtree insertion order), `gGourceRemovedFiles` vector, captions deque, active_captions list. Quadtree query results in `std::set<QuadItem*>` ordered by pointer: picking is nondeterministic when overlapping; quadtree rebuilt every frame so force results may depend on insertion order and max_depth (1 vs 6 switching at area 10000).
7. `calcEdges()` runs in draw path (2118) regardless of hide_tree; `updateBloomVBO/updateFilesVBO(dt)` run from draw path (2287, 2293) and are skipped when hide_bloom/hide_files; check in rdirnode.cpp whether they mutate sim state (they take dt). `checkFrustum`/`calcScreenPos` set visibility flags in draw path.
8. Draw/logic coupling: mouse picking (in draw) sets selection which drives camera tracking in logic.
9. changeColours uses libc rand() (platform-dependent); seed range 1..10000.
