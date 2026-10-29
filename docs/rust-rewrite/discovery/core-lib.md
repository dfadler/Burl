# Core library catalogue (src/core, acaudwell/Core submodule)

Scope: everything in `src/core/` except settings, conffile, seeklog, regex, stringhash, ppm, png_writer (other children).
All paths relative to repo root. Line refs are to the checked-out submodule.

## 0. Headline facts for the rewrite

| Fact | Evidence |
|---|---|
| GL context is **compatibility profile**, no version requested; code is fixed-function GL 1.x/2.x with optional GLSL 1.10-style shaders (`gl_Vertex`, `varying`, `ftransform()`), VBOs for text only | src/core/display.cpp:254, data/shaders/*.vert |
| SDL2 only in practice (SDL1.2 `#else` branches are dead; configure requires sdl2) | configure.ac:39, display.cpp:`SDL_VERSION_ATLEAST` blocks |
| GLEW is the loader; GLU used (`gluPerspective`, `gluProject`, `gluUnProject`, `gluBuild2DMipmaps`, `gluErrorString`) | gl.h:4, display.cpp:~510-575, texture.cpp:232 |
| Fixed-function path (`--ffp`) is a first-class mode: shaders/VBO-text only used when `!gGourceSettings.ffp` | src/gource.cpp:81-86, 2491-2530 |
| Not compiled into gource by Makefile.am: `ui/`, `tga.cpp`, `timer.cpp`, `utf8/` (header-only, used once), `ftgl/` (header only, used by fxfont). `tga.cpp` is listed only in gource.pro:84. No Gource source includes `core/ui/*` | Makefile.am:11-32, gource.pro:84, grep UIElement src (none) |
| `mktime_utc` is used only by core/settings.cpp (other child); svn.cpp has its own copy of the same TZ hack and prefers `timegm` | settings.cpp:331,348; src/formats/svn.cpp:118-144,239-241 |
| `GLTimer` (GL_TIME_ELAPSED query) is unused by Gource | grep (only timer.*) |
| Fixed values worth preserving: precache set, glyph page 512 max, padding 3, quadtree depth 6/items 1, min frame delta 16 ms in Gource (GourceShell sets min_delta_msec=16, gource_shell.cpp:29; SDLApp default is 8) | sections below |

## 1. File inventory

| File | Purpose | In gource build |
|---|---|---|
| gl.h | includes `<GL/glew.h>`; `glCheckError()` macro (PRINT_GL_ERRORS always defined) -> warnLog with gluErrorString | yes |
| display.cpp/.h | `SDLAppDisplay display` global: window, GL context, 2D/3D modes, project/unproject | yes |
| sdlapp.cpp/.h | `SDLApp` main loop base class, init (dirs), arg parsing, clipboard, Win32 console | yes |
| fxfont.cpp/.h | FreeType glyph-atlas font renderer + `FXFontManager` | yes |
| ftgl/FTUnicode.h | `FTUnicodeStringItr<T>` UTF-8/16/32 decoder (from FTGL, MIT) | header |
| vbo.cpp/.h | `VBO`, `quadbuf` (batched textured quads, GL_QUADS) | yes |
| shader.cpp/.h | GL program wrapper (`Shader`, `ShaderPass`, `ShaderManager`) | yes |
| shader_common.cpp/.h | GL-agnostic shader source preprocessor + typed uniforms (`AbstractShader*`) | yes |
| texture.cpp/.h | `TextureResource`, `TextureManager` | yes |
| tga.cpp/.h | `TGAWriter`, threaded `TGAExporter` (frame export as TGA) | NO (qmake only); Gource uses PPM/PNG writers |
| quadtree.cpp/.h | `QuadTree`/`QuadNode`/`QuadItem` spatial index | yes |
| bounds.h | `Bounds2D`, `Bounds3D` | yes |
| frustum.cpp/.h, plane.cpp/.h | 6-plane view frustum, plane | yes |
| pi.h | `PI 3.14159265`, `RADIANS_TO_DEGREES 57.29577951`, `DEGREES_TO_RADIANS 0.017453292` (low-precision constants; note for numeric parity) | yes |
| vectors.h/.cpp | GLM typedefs, `lerp2`/`lerp3`, `normalise` overloads | yes |
| timezone.cpp/.h | `mktime_utc` via TZ env hack | yes |
| timer.cpp/.h | `GLTimer` | NO |
| resource.cpp/.h | `Resource` refcount base + `ResourceManager` | yes |
| logger.cpp/.h | leveled logger | yes |
| mousecursor.cpp/.h | `MouseCursor` state + optional textured cursor | yes |
| ui/* | widget toolkit (unused by Gource) | NO |
| utf8/* | utf8-cpp (Nemanja Trifunovic) headers | `utf8.h` included only by gource_settings.cpp:25 (utf16to8 at :836, Windows arg path) |

## 2. display.cpp / display.h (global `display`)

Fields (display.h:~44-80): `enable_alpha, resizable, fullscreen, frameless, experimental, high_dpi_aware, vsync`, `zbuffer_depth` (default 16), `multi_sample` (0), `width,height` (GL viewport / drawable px), `desktop_width/height`, `windowed_width/height`, `viewport_dpi_ratio` (vec2, drawable/window), `sdl_window`, `gl_context`, `framed_width/height/x/y`, `clear_colour` (default 0,0,0,1).

Window/GL setup (`setVideoMode`, display.cpp:184-266):
| Step | Detail |
|---|---|
| GL attrs | DOUBLEBUFFER=1, DEPTH_SIZE=zbuffer_depth(16), MULTISAMPLEBUFFERS=1/SAMPLES=N only if `multi_sample>0` (Gource passes 4 when `--multi-sample`, main.cpp:134), ALPHA_SIZE=8 only if transparent |
| Window flags (:81-95) | `SDL_WINDOW_OPENGL|SHOWN`; `BORDERLESS` if frameless; `RESIZABLE` only if resizable && !frameless; `FULLSCREEN` (real mode, not DESKTOP) if fullscreen at creation; `ALLOW_HIGHDPI` if high_dpi_aware and not `_WIN32` |
| Position | no prior window + `screen` in 1..numDisplays: `SDL_WINDOWPOS_UNDEFINED_DISPLAY(screen-1)`; else CENTERED. Re-creation preserves display index and position |
| MSAA fallback | if CreateWindow fails and multi_sample>0, retry with MSAA off (:182-195) |
| Context | `SDL_GL_CONTEXT_PROFILE_MASK = COMPATIBILITY` set *after* window creation, then `SDL_GL_CreateContext` (:254-256); no major/minor version set |
| Swap | `SDL_GL_SetSwapInterval(vsync?1:0)` (:265) |
| Then | `glewInit()` (`glewExperimental` if `experimental`; ignores `GLEW_ERROR_NO_GLX_DISPLAY` for Wayland, :134-148) |
`init()` (:588-): `SDL_Init(TIMER|VIDEO)`; Windows: `setWindowsHighDPIAwareness()` via runtime-loaded `SetProcessDpiAwareness(PER_MONITOR)`/`SetProcessDPIAware`; invalid `screen` -> -1; desktop size from `SDL_GetDisplayBounds(screen-1 or 0)`; if width/height 0: fullscreen -> desktop size, else desktop size per axis; `atexit(SDL_Quit)`; then reads real `GL_VIEWPORT` into width/height, `SDL_GL_GetDrawableSize`, updates `viewport_dpi_ratio`, `glViewport`.
`toggleFullscreen` (:355): uses `SDL_WINDOW_FULLSCREEN_DESKTOP` (borderless desktop) at runtime, saves/restores windowed size; viewport from drawable size.
`toggleFrameless` (:402): macOS: no-op (`__APPLE__` early return, still compiles rest); ignored when fullscreen; toggles bordered, recreates window via `setVideoMode` (to drop RESIZABLE border); `_WIN32` block computes frame deltas with `GetWindowRect` (+2px HACK when resizable).
`resize` (:~495): re-reads drawable size and dpi ratio, `glViewport`.
`quit` (:662): delete context/window; purge texture/shader/font managers; `fontmanager.destroy()`.
2D/3D (:~680-725): `mode3D` = `gluPerspective(fov, w/h, near, far)`; `mode2D` = `glOrtho(0,w,h,0,-1,1)` (y down); `push2D/pop2D` push both matrices. `project()` flips Y (`viewport[3]-winY`); `unproject()` reads depth with `glReadPixels(GL_DEPTH_COMPONENT)` at the pixel (Y flipped) -> needs a depth buffer readback. `currentColour()` = `glGetFloatv(GL_CURRENT_COLOR)`.
`clear()`: `glClearColor(clear_colour)`, clears COLOR|DEPTH. `setClearColour(vec3)` alpha = enable_alpha ? 0 : 1.
Windows-only: `window_filter_proc` swallows `WM_SYSCOMMAND/SC_KEYMENU` to stop Alt "ding" (:163-166, 316-322).
Gource-side use: main.cpp:131-174 (frameless, multisample 4, alpha if transparent, vsync, resize unless PPM output, high_dpi default true except non-Windows when a viewport was specified without `--high-dpi`, `SDL_SetHint(MINIMIZE_ON_FOCUS_LOSS,"0")`, optional `SDL_SetWindowPosition` for window_x/y).

## 3. sdlapp.cpp / sdlapp.h

| Item | Detail |
|---|---|
| Globals | `gSDLAppResourceDir, gSDLAppConfDir, gSDLAppPathSeparator ("/" or "\\"), gSDLAppTitle, gSDLAppExec`; `gSDLAppShaderDir` in shader_common.cpp:9 |
| `SDLAppInit(title, exec, exepath)` (:178-260) | Resource lookup order below. Sets `texturemanager/fontmanager/shadermanager` dirs, calls `fontmanager.init()` (FreeType init) |
| `SDLAppParseArgs` (:262-311) | `-f`, `-w`, `-WxH` / `--WxH` (any leading dashes, last `x`) set fullscreen/size; others passed through |
| `SDLApp::run` (:405-465) | init(); `SDL_StopTextInput()`; loop: `delta=SDL_GetTicks()` diff, **accumulate until >= `min_delta_msec` (8)**, else `SDL_Delay(1)`; `t`,`dt` seconds float from total_msec; fps counter updated each 1000 ms; poll all events -> `handleEvent`; `update(t,dt)`; `display.update()` (swap). So max ~125 logical frames/s, dt granularity 1 ms |
| `handleEvent` (:343-403) | SDL_QUIT->quit(); MOUSEMOTION and MOUSEBUTTON x/y(+rel) multiplied by `display.viewport_dpi_ratio`; TEXTINPUT, TEXTEDITING, MOUSEWHEEL, WINDOWEVENT RESIZED -> `resize(data1,data2)`; KEYDOWN/UP -> `keyPress`. Other events ignored |
| Clipboard | `SDL_GetClipboardText/SetClipboardText` |
| Info/Quit | `SDLAppInfo` prints, exit(0); `SDLAppQuit` SDL_Quit, prints "`exec: msg`" + "Try '`exec` --help' for more information.", exit(1) |
| `SDLAppException(fmt,...)` | printf-formatted, 64 KiB buffer, `showHelp` flag |
| Platform ifdefs | `_WIN32`: include windows.h, `_WIN32_WINNT 0x0501`, console create/hide/resize (GetConsoleWindow, "<title> Console" naming, disable close button), "Press Enter" pauses unless existing console (:49-153); `_MSC_VER` skip unistd; `USE_X11` declares clipboard event filter in header only (no impl in .cpp); `SDL_VERSION_ATLEAST` dead branches |

### Resource directory lookup order (SDLAppInit)
1. Defaults: `conf_dir=""`, `resource_dir="data/"`, `fonts="data/fonts/"`, `shaders="data/shaders/"`.
2. Non-Windows: if `getcwd` ok, all four prefixed by CWD + "/". Windows: directory of `GetModuleFileName` (conf_dir = exe dir; note doubled `\\` separators).
3. `#ifdef SDLAPP_CONF_DIR` and dir exists -> conf_dir (not defined by Makefile.am).
4. `#ifdef SDLAPP_RESOURCE_DIR` (Makefile.am:67 = `$(pkgdatadir)`): if dir exists -> resource=that, fonts=`<dir>/fonts/`, shaders=`<dir>/shaders/`; **else** if `exepath` has a "/" -> `<exe dir>/data/{,fonts/,shaders/}`.
5. `#ifdef SDLAPP_FONT_DIR` (Makefile.am:85 = `$(gourcefontdir)`) and exists -> fonts dir overridden.
6. All get trailing slash (`SDLAppAddSlash`). Texture paths: relative -> `resource_dir + name`; absolute (`/x` or `X:`) used as-is; `grabFile` (external) never prefixed. Fonts same absolute test (fxfont.cpp:642). Shaders: `shader_dir + prefix + .vert/.frag/.geom/.comp`; `#include "x"` -> `gSDLAppShaderDir + x`.
`ResourceManager` (resource.cpp): `fileExists` = `boost::filesystem::is_regular_file`, `dirExists` = `is_directory`; refcounted `Resource` map keyed by name; `release()` deletes at refcount<=0; `purge()` deletes all.

## 4. Fonts: fxfont.cpp / fxfont.h

Approach: FreeType 2 -> 8-bit coverage bitmaps -> packed into **GL_ALPHA** atlas pages -> drawn as textured quads (immediate mode `GL_QUADS` in FFP, or batched into `quadbuf` + shader in shader mode).

| Aspect | Detail (fxfont.cpp) |
|---|---|
| Face load | `FT_New_Face(lib, path, 0)` per (file,size) `FXGlyphSet`, cached in `map<file, map<size,FXGlyphSet*>>` (:640-667). `grab(file,size,dpi=72,ft_flags=FT_LOAD_DEFAULT)`; no kerning, no hinting override, no bold/italic |
| Size | `FT_Set_Char_Size(face, 64*size, 64*size, dpi, dpi)` (:241-243); **size in points at 72 dpi == pixels**; `font_scale` applied by Gource callers (gource.cpp:50-67) |
| Metrics | `unit_scale = x_ppem/units_per_EM`; ascender/descender = face value * `unit_scale.y` (descender negative!); `FXFont::getHeight = ceil(asc+desc)`; `getMaxWidth = max_advance/64`; `getMaxHeight` = max glyph metric height seen so far |
| Glyph render | `FT_Load_Glyph` flags, `FT_Get_Glyph`, `FT_Glyph_To_Bitmap(FT_RENDER_MODE_NORMAL, origin 0, destroy=1)` (:81). `dims = bitmap w,h + (2,2)`; `corner = (left, -top) + (0.5,-0.5)`; `advance = (adv.x>>6, adv.y>>6)` **integer-pixel advance** (integer shift of 26.6) |
| Glyph lookup | `FT_Get_Char_Index` (no fallback if missing -> glyph 0 .notdef); lazy creation, cached in `map<codepoint,FXGlyph*>`; text decoded by `FTUnicodeStringItr<unsigned char>` UTF-8 (no invalid-sequence protection, ftgl/FTUnicode.h:184) |
| Atlas | `FXGlyphPage(w,h)`: W=H=`min(512, GL_MAX_TEXTURE_SIZE)` (:293-297); byte buffer zero-init; cursor starts (1,1); **row packing**: advance cursor by `width+padding(3)`, wrap row when `x+w+3 > W`, new row y += `max_glyph_height + 3`; fail if y+h+3 > H -> new page; texcoords inset `(x-0.5)/W .. (x+w+1.5)/W` (covers the 1px border of dims+2) |
| Texture | `texturemanager.create(W,H,false mipmaps,GL_CLAMP_TO_EDGE,GL_ALPHA,data)`; update via `texture->reload()` (full reupload) whenever a new glyph lands (`updateTexture`), except during precache |
| Precache | on `FXGlyphSet::init`: `0-9a-zA-Z;:'",<.>/?-_=+!@#$%^&*()\ ` plus space (:249) |
| Tab | `\t` in non-VBO draw advances `advance('M') * tab_width(4.0)` (:382-386); VBO path (`drawToVBO`) has **no** tab handling (tab maps to a glyph) |
| Draw (FFP, `use_vbo=false`) | `glColor4fv(colour)`, `glPushMatrix; glTranslatef(x,y)`; per glyph batch: `glBindTexture`, `glBegin(GL_QUADS)` with texcoord+vertex per corner (:363-406) |
| Draw (VBO, `use_vbo=true`) | glyph quads appended to `fontmanager.font_vbo` (`quadbuf`) with per-vertex colour; flow `startBuffer()` (reset, use_vbo=true) -> draw calls -> `commitBuffer()` (upload GL_DYNAMIC_DRAW, use_vbo=false) -> bind text shader -> `drawBuffer()` (gource.cpp:2491-2530) |
| `FXFont` options | `dropShadow(bool)`, `shadowStrength` (default 0.7), `shadowOffset` (default 1,1), `roundCoordinates`, `alignTop` (default true: y += height), `alignRight` (default false: x -= width), colour/alpha. **Shadow**: FFP draws text twice (black, alpha = colour.a*strength, offset); VBO mode skips and relies on the `text.frag` shader doing a shadow pass using `texel_size = 1/min(512,max_tex)` offset (data/shaders/text.frag) |
| No outline | no stroker; "shadow" is the only effect |
| `print(x,y,fmt,...)` | vsnprintf 4096 buffer |
| Manager | `unload()` unloads VBO only; `reload()` is a no-op (glyph textures reloaded via texturemanager); `destroy()` FT_Done_FreeType; `drawPages()` debug dump of atlas |
| Default font | `FreeSans.ttf` in data/fonts (gource_settings.cpp:36,446); user `--font-file` canonicalised |

Rewrite notes: advances are integer pixels (not subpixel) and glyph origin offsets use +0.5 bias; reproducing identical layout needs FreeType (same hinting default `FT_LOAD_DEFAULT`, i.e. native/auto hinting at 72 dpi). Layout widths feed UI (e.g. caption/key placement) so width parity matters more than raster parity.

## 5. vbo.cpp / vbo.h

| Item | Detail |
|---|---|
| `VBO` | thin `glGenBuffers/glBindBuffer/glBufferData/glBufferSubData`; reallocates only when `capacity < item_count`, else SubData |
| `quadbuf_vertex` | 32 bytes: `vec2 pos; vec4 colour; vec2 texcoord` (offsets 0/8/24) |
| `quadbuf::add(tex,pos,dims,colour[,texcoord])` | 4 vertices in order TL,TR,BR,BL; default texcoord (0,0,1,1); grows capacity x2 (`vertex_count*2`); records `quadbuf_tex{start_index,textureid}` run when texture changes (textureid>0) |
| `draw()` | client arrays (`glEnableClientState` VERTEX/COLOR/TEXCOORD), `glDrawArrays(GL_QUADS)`; one draw per texture run, bind with `glBindTexture(GL_TEXTURE_2D)`; every non-final run draws `end-start+1` vertices (one vertex past a multiple of 4; GL_QUADS truncates it) |

## 6. Shaders: shader.cpp/.h and shader_common.cpp/.h

Shader files: `data/shaders/{bloom,shadow,text}.{vert,frag}` — all **GLSL 1.10 compat** (no `#version`, `varying`, `gl_Vertex`, `gl_MultiTexCoord0`, `gl_TexCoord[]`, `gl_FrontColor`, `ftransform()`, `texture2D`). They are used only when `--ffp` is not set.
- bloom: `pos = gl_Vertex.xyz - gl_MultiTexCoord0.yzw` (tex coords carry centre in .yzw and radius in .x); frag: hash noise `fract(sin(dot(pos.xy,(11.3713,67.3219)))*2351.3718)`, radial `cos(length(pos*2)/radius)` falloff with smoothstep; output `gl_Color*gradient`.
- shadow: samples `tex` alpha * `shadow_strength`, black rgb.
- text: composites colour glyph with a `texel_size`-offset shadow copy (alpha-over), `combined = 1-(1-shadow)(1-colour)`.
Uniforms Gource sets: `tex`(sampler2D=0), `shadow_strength`(0.7), `texel_size` (gource.cpp:2523-2526).

### Shader class model (shader.cpp)
- `ShaderManager::grab(prefix)` refcounted; `manage()` register a hand-built shader; `unload()`/`reload(force)` (force reloads files from disk — used by gource_shell.cpp:123 for full reset).
- `Shader::loadPrefix()` (:235-276): looks for `.vert .frag .geom .comp`; needs a vertex shader unless compute exists else `ShaderException`. `load()` = `compile()` + `link()`: `glCreateShader`, `glShaderSource` (single string), `glCompileShader`, `glCreateProgram`, attach, `glLinkProgram`, then **shader objects deleted after link**. Types supported: vertex, geometry, fragment, compute (`GL_COMPUTE_SHADER` requires GL 4.3 headers in GLEW, only if files exist).
- Errors: compile/link logs -> `ShaderException(message, source)`; warnings logged at LOG_LEVEL_WARN; `errorContext()` regex-parses NVIDIA (`file(line) : error`), AMD/Mesa (`ERROR: 0:line:` / `0:line(col): error`) log formats and shows +-3 source lines (shader_common.cpp:15-19, 908-945).
- `use()`: pedantic warn on uninitialised uniforms; if `dynamic_compile && needsCompile()` (a **baked** uniform changed) recompile; then `glUseProgram` and upload non-baked uniforms every call (`applyUniform` per type: int/float/bool/sampler1-3D/vec2-4/mat3-4/int[]/float[]/vec2-4[] arrays). Unknown uniform location -> silent skip (pedantic log). `glUseProgram(0)` on unbind (gource.cpp also calls `glUseProgramObjectARB(0)`).

### shader_common preprocessor language (per-line, applied to every file/source)
Regexes (shader_common.cpp:11-19) matched in `AbstractShaderPass::preprocess(line)` (:1049-1089):
| Directive / construct | Effect |
|---|---|
| `#version N` | stored in pass `version`; re-emitted first by `toString` as `#version N` (omitted when 0) |
| `#extension NAME : enable|require|warn|disable` | stored in map; emitted after `#version` |
| `#include "file"` | recursive include of `gSDLAppShaderDir + file` (no cycle check, no relative-to-includer resolution); lines are themselves preprocessed |
| `uniform <type> <name>;  // optional comment` (also `name[N]`) | **removed from source** and registered as a typed `ShaderUniform` on the parent shader (shared across passes by name; duplicate across stages links to the same object); trailing `// comment` stored. Supported types: float,int,bool,sampler1D/2D/3D,vec2,vec3,vec4,mat3,mat4; arrays: int,float,vec2,vec3,vec4 (others -> `ShaderException`) |
| everything else | appended verbatim |
Output assembly (`toString`, :947-961): `#version` + `#extension` lines + each uniform's `write()` + rest of source. A uniform's `write()` emits `uniform T name;` normally, or, when **baked**, `#define NAME <literal>` (float `%e`, int `%d`, bool true/false, vec/mat constructor literals) / array as `T[N] name = T[](...)` const initialiser. Samplers are never baked. Setting a baked uniform to a new value marks it modified -> triggers recompile (`needsCompile`) when `dynamic_compile`.
Additional `Shader::addSubstitute(name,value)` / `ShaderPart::substitute`: plain text find/replace over the assembled source before compile (`applySubstitutions`). `ShaderPart` (`define()` -> prepended `#define`, per-line `substitute`) is a second, older mechanism and is not used by `Shader`.
Rewrite implication: Gource's three shader pairs use none of include/baked/substitute; a Rust port can use plain WGSL/GLSL with fixed uniforms and ignore the preprocessor, unless user shader support is wanted (it is not exposed in the CLI).

## 7. Textures: texture.cpp/.h

| Aspect | Detail |
|---|---|
| Loader | `IMG_Load` (SDL2_image; configure requires PNG and JPG support, configure.ac:45-46) — also decodes the `.tga` bloom textures |
| Formats | 4 bytes/px -> `GL_RGBA` if `Rmask==0x000000ff` else `GL_BGRA`; 3 bytes -> `GL_RGB`/`GL_BGR`; anything else (1/2 bytes) -> format 0 -> `TextureException`. 8-bit **indexed** surfaces are first converted `SDL_ConvertSurfaceFormat(RGBA32)` (:263-270, newest commits) |
| Upload | internalFormat: `GL_ALPHA` and `GL_LUMINANCE` kept, else `GL_RGBA`; `mipmaps` -> **`gluBuild2DMipmaps`** (GLU, any NPOT size) else `glTexImage2D` level 0; `GL_UNSIGNED_BYTE`. No `glPixelStorei(UNPACK_ALIGNMENT)` anywhere (RGB images with width*3 not multiple of 4 and GL_ALPHA atlas rely on 512 width) |
| Filter | no mipmaps: LINEAR/LINEAR. Mipmaps: min `GL_LINEAR_MIPMAP_LINEAR` (trilinear; `texturemanager.trilinear` default true) else `LINEAR_MIPMAP_NEAREST`, mag LINEAR. No anisotropy |
| Wrap | `GL_CLAMP_TO_EDGE` default, both S and T; callers pass explicit wrap |
| Targets | `create(GLenum target)` empty texture; `create(w,h,mipmaps,wrap,format,data)` raw (used for font pages and the `transition_texture` of size display w x h GL_RGBA in gource_shell.cpp:43) |
| Lifecycle | `grab(name,mipmaps=true,wrap)` refcounted by filename; `unload`/`reload` loops used on context-loss style resets (gource_shell.cpp:56-124); `release()` via ResourceManager |
| Gource callers | `file.png`, `user.png` (mipmaps, clamp), `beam.png`, `bloom.tga` / `bloom_alpha.tga` (alpha variant when `--transparent`), user avatars via `grabFile(path, mipmaps, clamp)`, logo (`mipmap_logo`), background image (gource.cpp:36-104; user.cpp:199) |

## 8. TGA writer (tga.cpp) — not built by autotools, documented for completeness
`TGAWriter(components=3|4)`: header image type 10 (RLE) / 2, 18-byte header, `short` width/height written in host byte order (little-endian assumed), bpp = comps*8, descriptor 8 if alpha else 0 (origin bottom-left, matching `glReadPixels(0,0,...)` row order, BGR/BGRA). RLE per scanline, packets <=128 pixels. `TGAExporter`: two buffers, SDL thread + cond/mutex handoff, `-` filename = stdout. Gource uses `PPMWriter`/`PNGWriter` instead (gource.cpp:2250).

## 9. Quadtree (quadtree.cpp/.h), bounds, frustum

Constants/behaviour:
| Item | Detail |
|---|---|
| Ctor | `QuadTree(bounds, max_node_depth, max_node_items)`; root depth 1 (node depth = parent_depth+1 and root created with parent_depth 0) |
| Gource parameters | `max_node_items = 1`; `max_depth = 1` initially, `gGourceMaxQuadTreeDepth = 6` (gource.cpp:23) only if `dir_bounds.area() > 10000.0`; tree is **rebuilt from scratch** (deleted + new) each update; bounds = item bounds expanded by 1.0 on each side (gource.cpp:1295-1312 users, 1425-1437 dirs) |
| Leaf accept rule | `allowMoreItems()`: no children AND (`depth >= max_depth` OR `items.size() < max_node_items`) (quadtree.cpp:38) |
| Subdivide | split at geometric centre into 4 children in order **TL(min..mid), TR, BL, BR** (:43-95); existing items redistributed to *every* overlapping child (items may live in several leaves; `item->node_count` tracks, `tree->item_count` counts duplicates, `unique_item_count` unique) then the new item |
| Placement | `addToChild`: item inserted into every child whose bounds `overlaps` item bounds (inclusive edges, `Bounds2D::overlaps` closed intervals) |
| Point query | `getChildIndex(pos)`: first child (order TL,TR,BL,BR) where `bounds.contains(pos)` (closed) -> boundary points go to earliest child; returns leaf items; **returns all items in a non-empty node without a point test** |
| Frustum/bounds query | recurse children where `!empty() && frustum.intersects(child.bounds)` (2D bounds at z=0) / `bounds.overlaps`; an inner node holding items returns them immediately (inner nodes never hold items after subdivision) |
| Output | results in `std::set<QuadItem*>` (pointer-ordered -> iteration order is allocation-dependent!) or visitor functors (deterministic traversal order TL,TR,BL,BR then list order) |
| Draw helpers | `generateLists`/`draw`/`outline`/`outlineItems` use GL display lists + `Bounds2D::draw` (debug only; display lists used here only, `glDeleteLists` in dtor) |
Rewrite note: query results are used for hit-testing/proximity; the set-vs-visitor choice affects tie-breaking only where Gource iterates the `std::set` (check callers in gource.cpp) — use deterministic ordering in Rust.

`Bounds2D` (bounds.h:~30-110): `min,max,first`; ctor from two points normalises through `update`; `update(point)` grows; `contains` closed & false if empty (`first`); `overlaps` closed; `centre/width/height/area`; `draw()` = GL_LINE_STRIP. `Bounds3D` similar (unused by quadtree).
`Frustum`: built from `source,target,up,fov(deg),near,far` using `display.width/height` aspect; 6 planes (top,bottom,left,right,near,far) from the 8 corners; `Plane(v1,v2,v3)`: `normal = normalize(cross(v3-v2, v1-v2))`, `d = -dot(normal, v2)`, `distance(p)=d+dot(n,p)` (inside >= 0). `intersects(Bounds3D)` / `intersects(Bounds2D, z=0)`: positive-vertex test per plane (conservative AABB test; may report false positives). Uses `DEGREES_TO_RADIANS` from pi.h.

## 10. Math (vectors.h / vectors.cpp / pi.h)
GLM with `GLM_FORCE_RADIANS`, `GLM_FORCE_CTOR_INIT` (zero-initialised vectors/matrices), `GLM_ENABLE_EXPERIMENTAL` (gtx/norm); `vec2/3/4`, `mat3/4` using-declarations. `normalise()` overloads (vectors.cpp) guard zero-length (verify when porting). `lerp2/lerp3`: vec + `p` (previous) + `l` (lerped); `snap()` copies current->p; `lerp(n)` = p + (cur-p)*n; assignment from plain vec keeps p. Single precision `float` throughout; pi constants are 8-9 digits (3.14159265 etc.) — use these exact constants if bit-level parity of angles matters.

## 11. timezone.cpp
`mktime_utc(tm*)`: `init_tz()` caches current `TZ` env once, `putenv("TZ=UTC"); tzset()`, `mktime()`, then restores `TZ` (or `unsetenv`, or `putenv("TZ=")` on `_WIN32`) and `tzset()`. Not thread-safe (mutates process env). `tm_isdst` semantics follow caller's struct (callers should zero it); under UTC DST never applies. Equivalent to `timegm`. Used by settings.cpp:331,348 (date args). svn.cpp:117-144 duplicates the hack (`__timegm_hack`) and uses `timegm` when available (svn.cpp:239-241). Rust: `chrono::NaiveDateTime::and_utc().timestamp()`.

## 12. Logger (logger.cpp)
Levels (enum order): OFF, ERROR, CONSOLE, INFO, SCRIPT, DEBUG, WARN, PEDANTIC; a message prints if `logger.level >= msg level`. Format `"%s: %s\n"` with 7-char right-aligned tags `"  ERROR"`, `"CONSOLE"`, `"   INFO"`, `" SCRIPT"`, `"  DEBUG"`, `"   WARN"`, `" PEDANT"` (:12-18). Default logger: `LOG_LEVEL_ERROR` -> stderr, history 0 (logger.cpp:~155). Printf-style (64 KiB stack buffer) and stream-style (`InfoLog() << a << b`, space-joined) APIs; optional history deque, auto-flush. Pedantic/warn levels gate shader diagnostics.

## 13. Mouse cursor (mousecursor.cpp)
State: pos/rel (viewport px), click flags (set by app), scrollwheel counter (+1/-1 per `scroll(dir)`), `idle` seconds, `timeout = 3.0`s, `hidden`, `system_cursor` (default true). Hidden cursor = 32x32 blank `SDL_CreateCursor`. `isVisible = !hidden && idle<timeout && SDL_GetMouseFocus()==display.sdl_window`. `leftButtonPressed` etc. poll `SDL_GetMouseState` live. `draw()` only if `!system_cursor && cursortex`: white `glColor4f`, `GL_TEXTURE_2D`+`GL_BLEND`, quad at mouse pos of texture w x h (translate is not pushed — leaks into current modelview; pushes after translate). Gource owns `MouseCursor cursor` (gource.h:73).

## 14. ui/ widget toolkit (unused by Gource)
Not referenced by any non-core source (no `UIElement`/`UIGroup` outside src/core/ui) nor listed in Makefile.am. Provided for Logstalgia/other apps. Classes: `UI` (manager), `UIElement` (base, type enum UI_ELEMENT/LABEL/BUTTON/IMAGE/LAYOUT/GROUP/COLOUR/SELECT/SLIDER/SCROLL_BAR...), `UILayout`/`UISolidLayout`/`UIScrollLayout`/`UIResizableLayout`, `UIGroup`/`UISubGroup`, `UILabel` (+Int/Float/Colour/String/Vec3 variants), `UIButton`, `UICheckbox` (+Bool/Float/Label sets), `UISlider` (Float/Int/Colour/Hue/Lightness/Sat), `UISelect`, `UIColour`, `UIImage`, `UIScrollBar`, `UIConsole`, `UIFileSelector`, `UIAction`. Draws with immediate-mode GL and `FXFont`. **Rewrite: skip.**
Gource's own on-screen widgets (slider.cpp, key.cpp, textbox, caption) live in src/ and use only FXFont/Texture/quad drawing, not this toolkit.

## 15. Rendering feature summary actually used by Gource through Core
| Feature | Where |
|---|---|
| `glMatrixMode/glLoadIdentity/glOrtho/gluPerspective/glPush/PopMatrix/glTranslatef` | display.cpp, gource.cpp |
| `glBegin(GL_QUADS)` immediate mode (many), `glTexCoord2f`, `glVertex2f/2i`, `glColor4f` | gource.cpp counts: glEnable 29, GL_TEXTURE_2D 27, glBegin 3 direct (rest in drawing classes) |
| Blend | `GL_BLEND` with `GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA` (14 uses) and additive `GL_SRC_ALPHA, GL_ONE` (2 uses) in gource.cpp; `glLineWidth` (2) |
| Depth | depth buffer 16 bit requested, cleared each frame; read back only for `unproject` |
| Shaders (non-FFP) | bloom/shadow/text via `ShaderManager`; `glUseProgram` |
| VBO | text only (`quadbuf`) with client-state arrays (GL 1.5-2.x style) |
| Display lists | only quadtree debug draw |
| Queries | `GLTimer` unused |
| Screenshots/video | `glReadPixels` in ppm/png writers (other children); frame size = `display.width/height` |

## 16. Platform ifdef index
| Ifdef | Location | Effect |
|---|---|---|
| `_WIN32` | sdlapp.h:34-39, sdlapp.cpp (console, dirs, separators), display.cpp (DPI awareness, WM filter, frameless rect math, no ALLOW_HIGHDPI), timezone.cpp (`TZ=` instead of unsetenv), main.cpp high-dpi rule | Windows specifics |
| `__APPLE__` | display.cpp:402-406 | frameless toggle disabled at runtime |
| `_MSC_VER` | sdlapp.cpp:33 | skip unistd.h |
| `USE_X11` | sdlapp.h | declares clipboard event filter (unimplemented here) |
| `SDL_VERSION_ATLEAST(2,0,0)` | everywhere | SDL1.2 fallback code dead under SDL2-only configure |
| `GLEW_ERROR_NO_GLX_DISPLAY` | display.cpp:~137 | Wayland tolerance |
| `USE_MGL_NAMESPACE` | shader.cpp:34 | skips gl.h include (unused) |
| `SDLAPP_CONF_DIR`, `SDLAPP_RESOURCE_DIR`, `SDLAPP_FONT_DIR` | sdlapp.cpp:215-244; Makefile.am:67,85 | install-path overrides |
| `PRINT_GL_ERRORS` | gl.h:6 | GL error macro always on |
| `HAVE_TIMEGM`-style | svn.cpp:239 | not Core |

## 17. Rust rewrite guidance (derived)
- Windowing: winit/SDL2 (sdl2 crate) + OpenGL (glow/glutin) compat profile is not available in core-profile-only stacks; visual equivalence is easiest with wgpu re-implementing fixed pipeline (orthographic 2D quads, perspective camera, alpha/additive blend, 16-bit depth for picking not required if `unproject` replaced by ray/plane math — Gource's `unproject` depth readback is used for mouse-to-world conversion, see callers).
- Keep: 16 ms minimum step (Gource overrides the 8 ms SDLApp default; see timing.md), float seconds `t,dt`, dpi-scaled mouse coordinates, FreeSans.ttf with integer advances, 512 px atlas pages, quadtree rules (depth 6 / 1 item / rebuild each tick / 1.0 padding / TL,TR,BL,BR order), logger tag format, resource lookup order, `--WxH`, `-f`, `-w` arg handling.
- Drop: ui/, tga, timer, shader preprocessor, SDL1.2 paths, Windows console shim (unless Windows target).

## Gaps
- `frustum.h` field list and `normalise` zero-guard in vectors.cpp not transcribed line-by-line.
- ui/ per-widget behaviour not catalogued beyond class list (unused).
- Callers of `QuadTree::getItemsAt/visit*` in gource.cpp not traced to see whether std::set ordering affects results.
- GL blend/state counts in gource.cpp only; user.cpp/file.cpp/dir.cpp/bloom draw code is for other children.

## Tick 2 additions

### (a) vectors.h / vectors.cpp
- `vectors.cpp:3-5` `rotate_vec2(v,s,c) = vec2(v.x*c - v.y*s, v.x*s + v.y*c)` (caller passes sin, cos).
- `vectors.cpp:7-23` all three `normalise` overloads (vec2/vec3/vec4) are identical in form:
  ```
  float l = glm::length(v);      // sqrt(dot(v,v)), f32
  if(l > 0.0) return v / l;      // component-wise DIVISION by l (not multiply by 1/l)
  return v;                      // zero vector -> returned unchanged; no NaN
  ```
  NaN length fails the test and returns v; +inf passes and yields NaN components. `glm::normalize` (used only in `plane.cpp:36`) is a different function (`v * inversesqrt(dot)`), not zero-guarded; it only feeds the draw-only frustum.
- `vectors.h:30-42`: `GLM_FORCE_RADIANS`, `GLM_FORCE_CTOR_INIT` (zero-init), `GLM_ENABLE_EXPERIMENTAL`; `using glm::vec2/3/4/mat3/mat4`. Includes `gtc/type_ptr`, `matrix_transform`, `matrix_access`, `gtx/norm`. Gource uses `glm::length2`, `length`, `cross`, `dot`.
- `lerp2`/`lerp3` (vectors.h:44-100): grep over `src/` finds NO use outside vectors.h. Dead code, do not port.
- No colour-conversion helpers exist in vectors.*; colours are plain `vec3/vec4` (hash colours in (e)).

### (b) Frustum (frustum.h:36-57, frustum.cpp)
Fields in order: `float near_distance, far_distance; float view_ratio; float fov;` (`fov` is declared, never assigned) `near_half_width, near_half_height, far_half_width, far_half_height;` eight corner `vec3`s `near_{top_left,top_right,bottom_left,bottom_right}`, `far_{...}`; `Plane planes[6]` (order: top, bottom, left, right, near, far).
- `updatePerspective`: `view_ratio = (float)display.width/(float)display.height`; `toa = (float)tan(fov*0.5*DEGREES_TO_RADIANS)` (pi.h:33 `0.017453292`); `near_half_height = near*toa; near_half_width = near_half_height*view_ratio`; same for far.
- `updateView`: `view_ray = normalise(target-source)`; `horiz = normalise(cross(view_ray, up))`; `vert = cross(horiz, view_ray)`; corners = centre +/- vert*half_h -/+ horiz*half_w. Planes (3 points each): top(`ntr,ntl,ftl`), bottom(`nbl,nbr,fbr`), left(`ntl,nbl,fbl`), right(`nbr,ntr,fbr`), near(`ntl,ntr,nbr`), far(`ftr,ftl,fbl`).
- `Plane(v1,v2,v3)` (plane.cpp:36-43): `normal = glm::normalize(cross(v3-v2, v1-v2))`, `point=v2`, `d=-dot(normal,point)`; `distance(p)=d+dot(normal,p)`.
- `intersects(Bounds2D b, float z=0)`: per plane pick corner `x = n.x>0 ? max.x : min.x`, same for y, z fixed; `distance<0` -> false; else true. `intersects(Bounds3D)` same with z picked; `contains(p)`: all distances >= 0.
- Usage in gource.cpp: ONE construction per drawn frame, `gource.cpp:2412` `Frustum(camera.getPos(), getTarget(), getUp(), getFOV(), getZNear(), getZFar())`, passed to `root->checkFrustum(frustum)` (gource.cpp:2437), which sets `RDirNode::in_frustum = frustum.intersects(quadItemBounds)` recursively (dirnode.cpp:1000-1008). `in_frustum` is read only by draw paths (dirnode.cpp:985 drawNames, 1012 drawShadows, 1036 updateFilesVBO, 1058 updateBloomVBO, 1079 drawFiles, 1152 drawBloom); the VBO builders only call `buffer.add`. QuadTree `*InFrustum` methods and `drawNodesInFrustum` have zero callers. Verdict: frustum culling is presentation-only and cannot affect simulation state.

### (c) QuadTree queries and iteration-order sensitivity
Rules (quadtree.cpp):
- `QuadTree(bounds, max_depth, max_items)`; root = `new QuadNode(tree, 0, bounds, 0)`; node ctor sets `depth = parent_depth+1`, so ROOT depth = 1 (leaves at depth >= max_depth are unbounded: max_depth 6 means 5 levels of subdivision below root). `allowMoreItems = children.empty() && (depth >= max_depth || items.size() < max_items)` (quadtree.cpp:36-39). Gource uses `max_items=1` and `max_depth=1` when `dir_bounds.area() <= 10000`, else 6 (gource.cpp:1305-1312 users, 1430-1437 dirs). With max_depth 1 the root is one unbounded leaf and ALL items are visited in insertion order.
- `QuadTree::addItem` sets `item->node_count = 0`, calls `root->addItem`, `unique_item_count++` (quadtree.cpp:496-500). `QuadNode::addItem`: if allowed, `items.push_back` (std::list), `item_count++`, `item->node_count++`. Else if it has children -> `addToChild`. Else subdivide: `middle = centre - min`, `relmax = max-min`; children in order [0] `(min, min+middle)`, [1] `(min+(middle.x,0), min+(relmax.x,middle.y))`, [2] `(min+(0,middle.y), min+(middle.x,relmax.y))`, [3] `(min+middle, max)`; existing items re-added in list order (`item_count--`, `node_count--`, `addToChild`), list cleared, then the new item `addToChild`. `addToChild` loops i=0..3 and adds to EVERY child with `bounds.overlaps(item->quadItemBounds)` (closed-interval overlap), so an item spanning k leaves sits in k lists (`node_count==k`); an item overlapping no child is silently dropped.
- Inner nodes hold no items after a split. `empty() = items.empty() && children.empty()`. `getChildIndex(pos)`: first i in 0..3 with `children[i].bounds.contains(pos)` (closed), else -1.
- `getItemsAt(set,pos)` (:135-155): if `items` non-empty insert all non-null into the set; else if no children 0; else descend ONE path via `getChildIndex(pos)` (-1 -> 0). `getItemsInFrustum/getItemsInBounds`: same shape, but descend into every child (index order 0..3) where `!child.empty() && (frustum.intersects(child.bounds) | bounds.overlaps(child.bounds))`; a leaf is returned wholesale with NO per-item overlap test. `visit*` variants are the same traversal but call the functor directly in list order instead of inserting into a `std::set`.
- Both trees are destroyed and rebuilt every sim tick: `interactUsers` (gource.cpp:1292-1333; users in `std::map<string,RUser*>` key order; bounds = `user_bounds` padded +-1.0) and `interactDirs` (gource.cpp:1418-1447; `gGourceDirMap` = `std::map<string abspath,...>` order; only `!node->empty()` nodes). Tick order (gource.cpp:1867-1873): `updateBounds; interactUsers; updateUsers; interactDirs; updateDirs`. Paused: `updateBounds; interactUsers; interactDirs` (gource.cpp:1694-1696).

Every QuadTree query caller (grep of all of src/):
| Caller | Query | Container | Consumer | Affects sim state? |
|---|---|---|---|---|
| gource.cpp:1326 `interactUsers` | `userTree->visitItemsInBounds(a->quadItemBounds, UserForceFunctor)` | none; leaf list order, children 0..3 | `RUser::applyForceUser(b)` (user.cpp:88-110) mutates only `this->accel` | YES, by order |
| dirnode.cpp:659 `RDirNode::applyForces` | `quadtree.visitItemsInBounds(quadItemBounds, DirForceFunctor)` | none; same order | `RDirNode::applyForceDir(d)` (dirnode.cpp:604-634) mutates only `this->accel` | YES, by order |
| gource.cpp:1899 `mousetrace` | `userTree->getItemsAt(userset, projected_mouse)` | `std::set<QuadItem*>` ordered by pointer value | first element in pointer order with `!isFading() && quadItemBounds.contains(mouse)` -> `userSelection` (break) | No; hover target only |
| gource.cpp:1915 `mousetrace` | `dirNodeTree->getItemsAt(dirset, projected_mouse)` | `std::set<QuadItem*>` pointer order | per dir, first file with `!isHidden() && overlaps(mouse)`; the `break` exits only the inner file loop, so the LAST dir in pointer order with a hit wins `fileSelection` | No; hover target only |
All other query methods (`getLeavesInFrustum`, `getItemsInFrustum`, `getItemsInBounds`, `visitItemsAt`, `visitLeavesInFrustum`, `visitItemsInFrustum`, `drawNodesInFrustum`, `generateLists`) have no callers; `outline` is debug draw (gource.cpp:2555).

ANSWER: pointer-ordered `std::set<QuadItem*>` iteration cannot change simulation state. The simulation never uses the set queries; the only set consumers are the mouse-hover picks (gource.cpp:1897-1935), where pointer-address order (non-deterministic under ASLR) only breaks ties between overlapping items for highlighting. Any deterministic tie-break is acceptable in a port.

BUT the simulation IS sensitive to the visit-LIST order of `visitItemsInBounds`, which a port must reproduce exactly:
1. `accel` is an f32 running sum (`accel -= (desired-dist)*normalise(dir)`, `accel += distance*normalise(dir)`); f32 addition is not associative, so neighbour visit order changes low bits, which then integrate into positions.
2. Overlap-resolution branches (`user.cpp:105-108` dist<0.001; `dirnode.cpp:622-626` posd<0.00001) call libc `rand()`; which node receives which value depends on visit order. `rand()` is not seeded in a default run (srand only at gource_settings.cpp:1319 for `--start-position random`), so results depend on the platform libc default-seeded sequence (glibc and macOS differ).
3. Dedupe: `UserForceFunctor` (user.h:98-126) and `DirForceFunctor` (dirnode.h:212-244) skip self; the dir functor also skips parent, direct children, ancestors and descendants (`isParent` chains). For items with `node_count != 1` a `std::set<T*> seen` applies the force only on the FIRST encounter in visit order (set is membership only, so its ordering is irrelevant). Leaves are visited wholesale, so far-away items are visited too; the force functions self-filter by distance (`distance2 > 0 -> return` for dirs, `dist < desired_dist` for users), which affects only the `loopCount` statistics.
4. Effects are local: functors write only the CALLING node's `accel`, consumed later in `logic`, so the order in which nodes run their queries (map order; dir recursion is children-first, dirnode.cpp:650-654) does not matter; only neighbour order within one query does.
Port rule: leaf `Vec` in insertion order; children order TL(0), TR(1), BL(2), BR(3); closed `overlaps`/`contains`; rebuild per tick from sorted-key map order; first-encounter dedupe. A brute-force O(n^2) loop is NOT equivalent (different summation order, rand() consumption order).

### (d) utf8 (src/core/utf8/*, utf8-cpp, header-only)
- Correction: gource_settings.cpp:25 only `#include`s `core/utf8/utf8.h`; its sole use is `utf8::utf16to8` at :836 inside `#ifdef _WIN32` (converts `p.filename().wstring()` for `--user-image-dir`; other platforms use `p.filename().string()` verbatim). `replace_invalid` is called in `formats/commitlog.cpp:29` inside `RCommitLog::filter_utf8` (commitlog.cpp:24-35) with replacement `'?'`, wrapped in `try{...}catch(...){ filtered = "???"; }`. Callers of `filter_utf8`: commitlog.cpp:292 (filename), :354 (username), gource.cpp:1112 (caption).
- `replace_invalid(start,end,out,'?')` (checked.h:99-125): loop while `start != end`; `err = validate_next(start,end)` (core.h:240-293):
  - `UTF8_OK`: copy consumed bytes unchanged.
  - `NOT_ENOUGH_ROOM` (truncated sequence at end): emit one `?`, `start = end`.
  - `INVALID_LEAD`: emit `?`, advance exactly 1 byte.
  - `INCOMPLETE_SEQUENCE` / `OVERLONG_SEQUENCE` / `INVALID_CODE_POINT`: emit one `?`, `++start`, then skip all following continuation bytes (`(b>>6)==2`); one `?` per bad sequence.
- Lead classification (core.h `sequence_length`): `<0x80`:1; `(b>>5)==6`:2; `(b>>4)==0xE`:3; `(b>>3)==0x1E`:4; else 0 = INVALID_LEAD (0x80-0xBF, 0xF8-0xFF). Each following byte must satisfy `(b>>6)==2`, else INCOMPLETE_SEQUENCE (or NOT_ENOUGH_ROOM at end). Decode: 2-byte `((b0<<6)&0x7ff)+(b1&0x3f)`; 3-byte `((b0<<12)&0xffff)+((b1<<6)&0xfff)+(b2&0x3f)`; 4-byte `((b0<<18)&0x1fffff)+((b1<<12)&0x3ffff)+((b2<<6)&0xfff)+(b3&0x3f)`. Then: code point invalid if `cp > 0x10FFFF` or in D800-DFFF -> INVALID_CODE_POINT; overlong if cp<0x80 and len!=1, cp<0x800 and len!=2, cp<0x10000 and len!=3 -> OVERLONG. NUL is valid and copied. F5-F7 leads decode >0x10FFFF -> INVALID_CODE_POINT.
- Rust: `String::from_utf8_lossy` is NOT equivalent (per-maximal-subpart U+FFFD vs one `?` per bad run). Implement the loop above.
- No other utf8 use in Gource sources (ftgl/FTUnicode.h and tinyxml carry their own decoders).

### (e) stringhash.cpp:28-83, exact
```
int gStringHashSeed = 31;      // reset to 31 gource_settings.cpp:489; --hash-seed sets it :1066; Gource::changeColours() sets (rand()%10000)+1 gource.cpp:1555
int stringHash(const std::string& str) {
    int val = 0;
    int n = str.size();                                  // size_t -> int
    for (int i = 0; i < n; i++)
        val = val + str[i] * (gStringHashSeed^(n-i));    // ^ is bitwise XOR, not power
    if(val<0) val = -val;
    return val;
}
vec2Hash:   h=stringHash(s); x=((h/7)%255)-127; y=((h/3)%255)-127; return normalise(vec2(x,y));
vec3Hash:   same x,y; z = h % 255;  return normalise(vec3(x,y,z));
colourHash: h=stringHash(s); if(h == 0) h++; r=(h/7)%255; g=(h/3)%255; b=h%255; return normalise(vec3(r,g,b));
```
- `str[i]` is plain `char`: SIGNED on x86/x64 and Apple arm64 (bytes >= 0x80 contribute negative), UNSIGNED on Linux aarch64/ppc. Assume signed (`b as i8 as i32`).
- The multiply and add are signed 32-bit and overflow (UB in C++, wraps in practice): use `wrapping_mul/add`. `-val` of INT_MIN stays INT_MIN; use `wrapping_neg`, so hash can be negative and `/`,`%` then truncate toward zero (matches Rust i32).
- Ints convert to f32 exactly; `normalise` = divide by `glm::length`, zero vector returned unchanged (vec2Hash gives (0,0) when x==y==0 after the offsets, e.g. when (h/7)%255 == 127 and (h/3)%255 == 127). Empty name: h=0 -> colourHash h=1 -> (0,0,1) normalised. Callers: dirnode.cpp:788/791 (initial dir offset, vec2Hash(abspath)), file.cpp:137 (ext colour; white if no ext), key.cpp:61, user.cpp:175 (`colourHash(name)`), commitlog.cpp:315.

### (f) Which src/core files are built, and what Gource calls
- Makefile.am:8-32 builds (src/core/): conffile, display, frustum, fxfont, logger, mousecursor, plane, ppm, quadtree, regex, resource, sdlapp, seeklog, settings, shader, shader_common, stringhash, texture, png_writer, timezone, vbo, vectors. gource.pro:65-87 builds the same plus `core/tga.cpp` (:84).
- NOT built by either: `core/ui/*` (no include of `ui/` in any Gource source), `core/timer.cpp` (`GLTimer`, no caller), `core/utf8/*` and `core/ftgl/*` (header-only, included). `tga.cpp` only in qmake and nothing uses it; `bloom.tga`/`bloom_alpha.tga` are installed data (Makefile.am:69) loaded via `texturemanager.grab` -> `IMG_Load` (SDL_image, texture.cpp:258).
- Correction to earlier text: `vbo.cpp` IS used by Gource: `quadbuf` (vbo.cpp:32-150: ctor(capacity), unload, resize, reset, vertices, capacity, texture_changes, `add` x3, update, draw) backs `file_vbo, user_vbo, edge_vbo, action_vbo` (gource.h:105-108) and text (fxfont.h:62); draw-only batching. `bloombuf` is in bloom.h, not core.
- Sim-relevant core calls: quadtree (c), stringhash (e), `normalise` (a), `Bounds2D` (bounds.h update/contains/overlaps), utf8 filter (d). Rendering/IO/platform only: frustum, plane, texture, display, shader, fxfont, mousecursor, ppm, png_writer, vbo, resource, sdlapp, logger, timezone.

## Tick 2 gaps
- Frustum/Plane f32 bit-level behaviour only matters for cull decisions (presentation).
- `accel` -> position integration in `RUser::logic`/`RDirNode::logic` not traced here (scene-model docs).
- Whether mouse hover/selection feeds camera-follow state was not traced.
- libc `rand()` sequence per platform is unspecified (see (c).2).
