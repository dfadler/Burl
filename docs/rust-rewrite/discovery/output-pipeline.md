# Output / rendering pipeline discovery

Scope: frame export (PPM/PNG), frame loop cadence, render pass order, GL state, shaders, assets.
Paths relative to repo root. Line numbers from this worktree.

## 1. PPM stream (`--output-ppm-stream` / `-o`)

| Item | Detail | Source |
|---|---|---|
| Option parse | `output-ppm-stream` string; `-` = stdout; empty = no export | src/core/settings.cpp:446-454 |
| Creation | `new PPMExporter(file)` after display init; failure -> `SDLAppQuit("could not write to '%s'")` | src/main.cpp:186-202 |
| Header | ASCII `"P6\n%d %d 255\n"` (width height), written before EVERY frame; no comment line | src/core/ppm.cpp:175, 186 |
| Body | width*height*3 bytes RGB8, rows top-to-bottom (GL bottom-up readback flipped in writer thread), no padding | src/core/ppm.cpp:107, 136-142, 187 |
| Readback | `glPixelStorei(GL_PACK_ALIGNMENT,1)`; `glReadPixels(0,0,w,h,GL_RGB,GL_UNSIGNED_BYTE)` from back buffer, after scene+UI drawn, BEFORE mouse cursor drawn, before swap. `dump()` first does `display.mode2D()`, `glEnable(GL_TEXTURE_2D)`, `glDisable(GL_BLEND)` | src/core/ppm.cpp:98-117; src/gource.cpp:271-281 |
| Alpha | None in PPM; `--transparent` only changes clear alpha | src/core/ppm.cpp:107 |
| Threading | One SDL thread "frame_exporter", mutex+cond, double buffer (pixels1/pixels2) + pixels_out. Main thread glReadPixels into the idle buffer, swaps `pixels_shared_ptr`, signals; worker flips rows and writes. If worker still writing, main blocks on mutex (no drop, no queue deeper than 1) | src/core/ppm.cpp:46-60, 96-151; src/core/ppm.h:35-54 |
| Stdout | `output=&std::cout`; `_WIN32`: `_setmode(_fileno(stdout), _O_BINARY)`. File: binary `std::ofstream`. No per-frame flush (stream buffering; closed at exit) | src/core/ppm.cpp:156-176, 180-188 |
| Rust notes | Same bytes via BufWriter<Stdout>; writer thread with bounded channel (cap 1-2) mimics blocking. Handle broken pipe cleanly (C++ ignores errors). |  |

### Frame cadence / fixed timestep

| Item | Detail | Source |
|---|---|---|
| `--output-framerate/-r` | int; only 25, 30, 60 else "supported framerates are 25,30,60"; default 60 | src/core/settings.cpp:456-470, 89 |
| Hookup | `Gource::Gource(exporter)` -> `setFrameExporter(exporter, output_framerate)` | src/gource.cpp:165, 505-522 |
| Tick calc | `gource_fps = video_fps; frameskip=0; while(gource_fps<60){gource_fps += video_fps; frameskip++;}`; `max_tick_rate = 1/gource_fps`. 60 -> sim 60 Hz, skip 0. 30 -> sim 60 Hz, dump every 2nd frame. 25 -> sim 75 Hz, dump every 3rd | src/gource.cpp:505-522 |
| Default | `max_tick_rate = 1/60`; `scaled_dt = min(real_dt, max_tick_rate)` | src/gource.cpp:156, 248 |
| With exporter | `scaled_dt = max_tick_rate` (wall-clock independent), then `*= time_scale`; `runtime += scaled_dt` unless paused | src/gource.cpp:246-259 |
| Order in `Gource::update` | stop_at_time check -> `logic(runtime, scaled_dt)` -> `draw(runtime, scaled_dt)` -> if exporter && commitlog && !shutdown && `framecount % (frameskip+1)==0` -> `dump()` -> cursor logic/draw (real dt) if !hide_mouse -> `framecount++` | src/gource.cpp:246-286 |
| Main loop | SDLApp::run accumulates real ms until >= `min_delta_msec` (GourceShell sets 16) so loop is still wall-clock throttled; sim dt fixed. `display.update()` swap each iteration. Window not resizable when ppm output set | src/core/sdlapp.cpp:390-460; src/gource_shell.cpp:29; src/main.cpp:147 |
| Rust notes | Can run unthrottled offscreen; keep: fixed dt formula, frameskip rule, dump after UI and before cursor, draw() also receives scaled dt. |  |

### Stop flags and the frame loop

| Flag | Behaviour | Source |
|---|---|---|
| exporter present (and not `--dont-stop`, `--loop`, path `-`) | forces `stop_at_end = true` | src/gource_shell.cpp:219-226 |
| `--stop-at-end` | stop_position_reached when log finished | src/gource.cpp:1170-1178 |
| `--stop-position F` | reached if seekable and (finished or last_percent >= F) | src/gource.cpp:1170-1178 |
| `--stop-at-time S` | `runtime >= S` (sim time, deterministic under export) | src/gource.cpp:261; src/gource_settings.cpp:1340-1349 |
| `--stop-timestamp` | commit.timestamp > stop_timestamp stops reading | src/gource.cpp:1143-1150 |
| After reached | App ends (`appFinished`) only when `users.empty() && stop_position_reached`, so trailing fade-out frames ARE exported; `file_idle_time_at_end` applied | src/gource.cpp:1180-1183, 1401-1403 |
| Multi-repo | stop_at_time defaults to 60 s if no stop set; no looping when exporting | src/gource_shell.cpp:228-241 |
| ESC/shutdown | `shutdown=true`: no more dumps; logic ends app when logmill finished | src/gource.cpp:272, 1570-1573 |
| GUI toggles | fullscreen/frame toggle disabled when exporter; slider hidden when exporter | src/gource_shell.cpp:50-75; src/gource.cpp:167 |
| `--stop-on-idle` | no-op | src/gource_settings.cpp:1376-1379 |

### `--output-custom-log`

No graphics: after config import `Gource::writeCustomLog(path, file)` then `exit(0)` (src/main.cpp:119-123). Section "command-line" (src/gource_settings.cpp:250, 329, 583-584). Writes `"%lld|%s|%s|%s\n"` = timestamp|username|action|filename per commit file; `-` = stdout else `fopen(...,"w")` (silently returns if fails); log error -> SDLAppQuit (src/gource.cpp:171-214).

## 2. PNG screenshot

| Item | Detail | Source |
|---|---|---|
| Trigger | F12 (only once commitlog exists) sets `take_screenshot`; executes near END of `draw()` after UI and debug text, before transient message text | src/gource.cpp:694-696, 2762-2765 |
| Naming | `gource-%04d.png` in CWD, first free 1..9999 by `stat()`; if all taken, overwrites `gource-9999.png` | src/gource.cpp:2234-2255 |
| Format | libpng 8-bit, RGB (3) or RGBA (4 if `--transparent`), no interlace, default compression/filter, rows flipped | src/gource.cpp:2250; src/core/png_writer.cpp:57-122 |
| Readback | synchronous `glReadPixels` GL_RGB/GL_RGBA, pack alignment 1 | src/core/png_writer.cpp:73-79 |
| Feedback | `setMessage("Wrote screenshot %s")`, 5 s | src/gource.cpp:2253, 2220-2232 |
| PNGExporter (threaded, `-` => stdout) | defined but not instantiated by gource (only PPMExporter, src/main.cpp:188-192); skip | src/core/png_writer.cpp:130-260 |

## 3. Display, clear, projection

| Item | Detail | Source |
|---|---|---|
| GL context | SDL2, `SDL_GL_CONTEXT_PROFILE_MASK = COMPATIBILITY`, double buffer, depth size, optional 4x MSAA (`--multi-sampling`, off by default), alpha 8 bit only if `--transparent`, vsync via swap interval | src/core/display.cpp:184-300; src/main.cpp:133-145 |
| Clear | `background_colour` default vec3(0.1,0.1,0.1) (#1A1A1A), alpha 1.0 (0.0 if `--transparent`); `glClear(COLOR|DEPTH)` once per frame in `drawBackground` | src/gource_settings.cpp:439, 1087-1100; src/gource.cpp:2025-2031; src/core/display.cpp:683-686 |
| Background image | `--background-image` via `grabFile`, no mipmaps, centred at native size, SRC_ALPHA/ONE_MINUS_SRC_ALPHA, white | src/gource.cpp:102-105, 2033-2059 |
| 2D mode | `glOrtho(0,w,h,0,-1,1)` (y down), identity modelview | src/core/display.cpp:696-702 |
| 3D scene | `gluPerspective(fov=90, w/h, znear=0.1, zfar=max+1)` + `gluLookAt(pos,target,up)`; start z=-300 | src/zoomcamera.cpp:23-26, 54, 75-82; src/gource.cpp:140 |
| Screen projection | `gluProject` of dir/file/user positions, y flipped by viewport height; used for 2D edges and text | src/dirnode.cpp:945-975; src/gource.cpp:2452-2476 |
| Depth test | never enabled in draw code; painter's order only | src/gource.cpp:2025-2780 |
| HiDPI | width/height = drawable size; mouse scaled by viewport_dpi_ratio; default font scale from dpi ratio or `1 + max(0, height/threshold)` | src/gource.cpp:39-47 |

## 4. Render pass order (`Gource::draw`, src/gource.cpp:2401-2780)

Shader mode (default `ffp=false`) is the reference; `--ffp` does the same via immediate mode (skip in Rust).

| # | Pass | GL state | Shader / uniforms | Geometry / vertex format | Source |
|---|---|---|---|---|---|
| 0 | `mode2D`; `drawBackground` (clear + optional image) | blend SRC_ALPHA,1-SRC_ALPHA | none | immediate quad | gource.cpp:2403-2409, 2025-2059 |
| - | no commitlog: `loadingScreen()` "Reading Log"/"Aborting" + 0-3 dots (`int(runtime*3)%4`), `font` 14*scale white at (w/2-tw/2, h/2-10); return | blend | none | text | gource.cpp:1991-2022 |
| 1 | mouse trace; PROJECTION=identity then `camera.focus()`; frustum cull; `calcScreenPos`; `updateVBOs` | | | | gource.cpp:2411-2490, 2256-2296 |
| 2 | Edges (skip if hide tree): temp ortho 2D (screen-space splines), blend SRC_ALPHA/1-SRC_ALPHA, bind beam.png. (a) shadow shader, tex=0, shadow_strength=0.5, translate (+2,+2) px, draw edge_vbo; (b) program 0, draw edge_vbo (texture*vertex colour) | blend, TEXTURE_2D | shadow.vert/.frag | quadbuf_vertex 32 B (pos vec2, colour vec4, uv vec2). Per spline segment a quad with half-width 2.5 px; uv.x 1 on +perp side, 0 on -perp; uv.y 0; per-point colour lerp child->parent dir colour; edge_detail = min(10, int(angle/PI*100)), min 1; quadratic Bezier via spos | gource.cpp:2116-2172; spline.cpp:23-135 |
| 3 | File shadows (skip if hide files): shadow shader tex=0 strength 0.5; bind file graphic; modelview translate (+2,+2) WORLD units; `file_vbo.draw()` | blend | shadow | quadbuf, one quad per visible file, colour (file col, alpha), uv 0..1 | gource.cpp:2298-2322 |
| 4 | Actions (skip if hide users): bind beam.png; fixed function | blend | none | quad: src end half-width 0.3*(target size/2) colour alpha 0.1a, dest end half-width target size/2 alpha a (a = 1-progress), uv (0,0),(0,1),(1,1),(1,0) | gource.cpp:2174-2194; action.cpp:40-65 |
| 5 | Files: bind file graphic, `file_vbo.draw()` fixed function | blend | none | as 3 | gource.cpp:2354-2374 |
| 6 | User shadows: shadow shader, offset (2,2)*user_scale world; `user_vbo.draw()` | blend | shadow | quadbuf, per-texture runs | gource.cpp:2324-2352 |
| 7 | Users: `user_vbo.draw()`; size `user->dims` (scaled by cam distance if `--fixed-user-size`) | blend | none | per-user avatar texture | gource.cpp:2376-2399, 2268-2277 |
| 8 | Bloom (skip if hide bloom): blend **ONE, ONE** additive, bloom shader, `bloom_vbo.draw()` | additive | bloom.vert/.frag, no uniforms | bloom_vertex 64 B: pos vec2 @0, colour vec4 @8, texcoord vec4 @24, 24 B pad; GL_QUADS. Per visible dir: R = dir_radius*2*bloom_multiplier; quad size 2R centred at dir pos; colour = dir colour*bloom_intensity, alpha 1; texcoord = (R, pos.x, pos.y, 0) | gource.cpp:2196-2218; dirnode.cpp:1056-1074; bloom.cpp:70-132 |
| 9 | Text (2D `push2D`): `startBuffer`; dir names (`fontdirname`, colour dir_colour); user names (not selected); `commitBuffer`; text shader tex=0, shadow_strength=0.7, texel_size=1/min(512, GL_MAX_TEXTURE_SIZE); draw buffer; then selected user/file names redrawn immediate with CPU shadow; `pop2D` | blend | text | glyph quads on A8 atlas | gource.cpp:2494-2570; fxfont.cpp:594-620 |
| 10 | debug overlays (`-q`, quadtree) | | | | gource.cpp:2572-2600 |
| 11 | 2D UI: logo (bottom-right: `(w,h)-(logo w,h)-logo_offset`); splash (first 15 s: bar colour (0,0.5,1, splash*0.015), "Gource" 42px, tagline, copyright); date (fontmedium, x = w/2 - date_x_offset, y=20); title (10, h-10, bottom-aligned); captions; file key; slider (if seekable); hover textbox; debug text; **screenshot**; transient message (font at (1,3)) | blend | none | immediate quads + text | gource.cpp:2615-2780 |
| 12 | back in `update`: PPM dump, then cursor | | | | gource.cpp:271-284 |

Notes
- Screenshot is inside step 11: includes UI, excludes cursor and the message text.
- Shadow constants: edges (2,2) px; files (2,2) world; users (2,2)*user_scale world; shadow strength 0.5 (`gGourceShadowStrength`, src/pawn.cpp:20); text CPU shadow offset (1,1) px alpha 0.7; GPU text shadow = 1 atlas texel up-left sample, strength 0.7.
- Shadow shader: out = (0,0,0, vertex.a * tex.a * 0.5).
- All passes SRC_ALPHA/ONE_MINUS_SRC_ALPHA (same factors for alpha channel) except bloom (ONE,ONE). With `--transparent`, framebuffer alpha is the result of those factors (matters for RGBA screenshots only).
- Bloom shader path ignores bloom.tga; FFP path uses it (`bloom.tga` or `bloom_alpha.tga` if transparent).
- Multi-repo transition: `glCopyTexImage2D` of last frame, blended back with alpha=transition_interval (1.0, decreasing by dt) (src/gource_shell.cpp:246-300).

## 5. Shaders (legacy GLSL, no `#version` line = GLSL 1.10 / compat profile)

Features used: `varying`, `gl_Vertex`, `gl_MultiTexCoord0`, `gl_Color`, `gl_FrontColor`, `gl_TexCoord[0]`, `gl_FragColor`, `ftransform()`, `texture2D`, `uniform sampler2D/float`, `smoothstep`, `fract`, `sin`, `dot`, `cos`, `length`, `min`. Loaded by prefix `<shader_dir>/<name>.vert|.frag` (src/core/shader.cpp:238-262), shader dir `<resource_dir>/shaders/` (src/core/sdlapp.cpp:221-226). Gource grabs "shadow", "bloom", "text" (src/gource.cpp:83-85).

### data/shaders/shadow.vert and text.vert (identical)
```glsl
void main(void)
{
  gl_TexCoord[0] = gl_MultiTexCoord0;
  gl_FrontColor  = gl_Color;
  gl_Position    = ftransform();
}
```
### data/shaders/shadow.frag (uniforms: tex, shadow_strength)
```glsl
uniform sampler2D tex;
uniform float shadow_strength;

void main(void)
{
    vec4 colour = texture2D(tex,gl_TexCoord[0].st);

    gl_FragColor = vec4(0.0, 0.0, 0.0, gl_Color.w * colour.w * shadow_strength);
}
```
### data/shaders/text.frag (uniforms: tex, shadow_strength=0.7, texel_size)
```glsl
uniform sampler2D tex;
uniform float shadow_strength;
uniform float texel_size;

void main(void)
{
    float colour_alpha = texture2D(tex,gl_TexCoord[0].xy).w;
    float shadow_alpha = texture2D(tex,gl_TexCoord[0].xy - vec2(texel_size)).w * shadow_strength;

    float combined_alpha = 1.0 - (1.0-shadow_alpha)*(1.0-colour_alpha);

    if(combined_alpha > 0.0) colour_alpha /= combined_alpha;

    gl_FragColor = gl_Color * vec4(vec3(colour_alpha), combined_alpha);
}
```
### data/shaders/bloom.vert
```glsl
varying vec3 pos;

void main()
{
    pos = gl_Vertex.xyz - gl_MultiTexCoord0.yzw;
    gl_TexCoord[0] = gl_MultiTexCoord0;
    gl_FrontColor = gl_Color;
    gl_Position = ftransform();
}
```
Vertex position is vec2 so `.xyz` = (x,y,0); texcoord = (R, cx, cy, 0) so `pos` = fragment offset from dir centre, in world units.
### data/shaders/bloom.frag (no uniforms)
```glsl
varying vec3 pos;

void main()
{
    float r = fract(sin(dot(pos.xy ,vec2(11.3713,67.3219))) * 2351.3718);

    float offset = (0.5 - r) * gl_TexCoord[0].x * 0.045;

    float intensity = min(1.0, cos((length(pos*2.0)+offset)/gl_TexCoord[0].x));
    float gradient  = intensity * smoothstep(0.0, 2.0, intensity);

    gradient *= smoothstep(1.0,0.67+r*0.33, 1.0-intensity);

    gl_FragColor = gl_Color * gradient;
}
```
Procedural, per-pixel hash grain `r`. Beyond pi/2 radius `cos` goes negative; the compat pipeline clamps gl_FragColor to [0,1], so the port must clamp output to >= 0 (otherwise additive blend would darken). `smoothstep(1.0, 0.67+r*0.33, x)` has edge0 > edge1 (undefined in GLSL spec; GPUs evaluate as the reverse ramp) - implement explicitly in WGSL as `1 - smoothstep(0.67+r*0.33, 1.0, x)`.

Not a shader asset: data/shaders contains exactly these six files (no README).

## 6. Assets and loading

| Asset | Where / how | Source |
|---|---|---|
| data/fonts/FreeSans.ttf (697 KB; fonts/README is FreeFont's) | default `font_file`; `fontmanager.grab(file,size)`; FreeType dpi 72, `FT_LOAD_DEFAULT`, `FT_RENDER_MODE_NORMAL`; glyph atlas pages min(512, GL_MAX_TEXTURE_SIZE)^2 A8, 3 px padding, texcoord -0.5/+1.5 px, LINEAR, CLAMP_TO_EDGE | src/gource.cpp:50-69; src/core/fxfont.cpp:62-100, 130-200, 292-310 |
| data/file.png 256x256 RGBA | `grab("file.png", mipmaps, CLAMP_TO_EDGE)` unless `--file-image` | src/gource.cpp:35-37 |
| data/user.png 384x512 RGBA | default avatar; mipmaps, clamp | src/gource.cpp:78; src/user.cpp:213 |
| data/beam.png 128x1 RGBA | edge/action beam; no mipmaps, default wrap | src/gource.cpp:77 |
| data/bloom.tga 512x512 RGB RLE; bloom_alpha.tga 512x512 RGBA RLE | FFP bloom only | src/gource.cpp:73-76 |
| user images (`--user-image-dir`, `--default-user-image`) | `grabFile(path, mipmaps, CLAMP_TO_EDGE)` | src/user.cpp:181-217 |
| `--logo`, `--background-image` | `grabFile` (logo mipmapped only if NPOT unsupported) | src/gource.cpp:96-105 |
| data/gource.style | Mercurial log template for `hg log --style`, not rendering | data/gource.style |
| resources/file.xcf, user.xcf | GIMP sources, unused at runtime | resources/ |
| Loader | `IMG_Load` (SDL_image) -> GL texture; mipmaps `gluBuild2DMipmaps`; mipmapped min = LINEAR_MIPMAP_LINEAR if trilinear else LINEAR_MIPMAP_NEAREST, mag LINEAR; else LINEAR/LINEAR | src/core/texture.cpp:160-260 |
| Resource dir | compile-time `SDLAPP_RESOURCE_DIR` (fonts/, shaders/) | src/core/sdlapp.cpp:215-235 |

## 7. Text sizes and UI constants

`font_scale` = 1 unless HiDPI default or `--font-scale`; `scaled_* = clamp(int(size*scale),1,100)` (src/gource_settings.cpp:523-526).

| Element | Size (default) | Notes | Source |
|---|---|---|---|
| Date / title (`fontmedium`) | `--font-size` 16 | shadow, no rounding | gource.cpp:54-56; gource_settings.cpp:447 |
| Dir names | `--dir-font-size` 14 | shadow, round | gource.cpp:67-69 |
| File names | `--file-font-size` 14 | | gource_settings.cpp:448 |
| User names | `--user-font-size` 14 | | gource_settings.cpp:450 |
| Captions | `--caption-size` 16 | | gource.cpp:58-61; gource_settings.cpp:482 |
| Splash | 42*scale | | gource.cpp:50 |
| Debug/loading/message | 14*scale | | gource.cpp:63 |
| Hover textbox | 18*scale; bg black alpha 0.333 | | gource.cpp:142; textbox.cpp:126 |
| Slider label | 16*scale | | slider.cpp:28 |
| File key | scaled_font_size; entry width 90*scale, height size+4; bg black 0.333 | | key.cpp:17-19, 121 |
| Bloom | `--bloom-multiplier` 1.0, `--bloom-intensity` 0.75 | | gource_settings.cpp:436-437 |
| Shadow strength | 0.5 (geometry), 0.7 (text) | | pawn.cpp:20; gource.cpp text pass |

## 8. Changes needed for a modern backend (wgpu) and visual-equivalence checklist

1. No fixed function: replace `ftransform`, `gl_Color`, `gl_TexCoord`, immediate mode, `GL_QUADS`, matrix stack, `gluPerspective/LookAt/Project`. Use a view-proj uniform (vertical fov 90, aspect w/h, near 0.1, far = camera zfar) and replicate `gluProject` maths on CPU (needed for edges, labels). Triangulate quads (0,1,2, 0,2,3).
2. Port to WGSL: pipelines for textured-tinted, shadow, text, bloom, solid-colour UI. Reproduce bloom hash; clamp output; fix the reversed smoothstep.
3. Blend: straight alpha with explicit same factors for colour and alpha; bloom One/One.
4. Framebuffer: offscreen `Rgba8Unorm` (not sRGB) at exact output size; readback with 256-byte row padding; drop alpha for PPM. Clear colour in non-linear space as given.
5. Text: FreeType hinted 72 dpi with integer-rounded origin when `roundCoordinates` is on; `freetype-rs` is closest; fontdue/ab_glyph will differ slightly. Keep FreeSans.ttf; embed with `include_bytes!` or install path.
6. Textures: PNG/TGA (RLE 24/32) plus arbitrary user images via `image` crate; generate mipmaps with trilinear sampling for file/user/avatar textures; beam.png wrap is GL default REPEAT.
7. No depth buffer needed.
8. Cadence exactly per section 1; independent of wall clock when exporting.
9. PPM exact bytes; header every frame; binary stdout.
10. Cursor, transient message and (for PPM) alpha excluded.
11. Pixel exactness not required: MSAA (off by default), glyph rasterisation, bloom noise, filtering may differ.

## 9. Gaps / not verified

- `Shader::use`/uniform plumbing and `src/core/shader_common.cpp` (src/core/shader.cpp:262-330; shader_common.cpp whole).
- FXGlyphSet pen advance / kerning and `drawToVBO` (src/core/fxfont.cpp:200-430) skimmed only.
- `drawNameText` (src/user.cpp:384-395, src/file.cpp) and `RDirNode::drawDirName` (src/dirnode.cpp ~900-940) label placement/alpha.
- `FileKey::draw`, `Slider::draw`, `TextBox::draw` full geometry (src/key.cpp:110-160, src/slider.cpp:120-170, src/textbox.cpp:120-160).
- Mouse cursor drawing (src/core/mousecursor.cpp), not exported.
- Pawn/file/user colour, size, alpha formulas (simulation fan-out; src/pawn.cpp).
- `--ffp` immediate-mode path only referenced; recommended to drop.

## Tick 2 additions

### T2.a Shader uniform plumbing (src/core/shader_common.cpp, src/core/shader.cpp)

- Declaration: uniforms are parsed from GLSL source by regex `^\s*uniform\s+(\w+)\s+(\w+)(?:\[(\d+)\])?\s*;\s*(?://\s*(.+))?$` (shader_common.cpp:14), typed by GLSL type (int, float, bool, sampler1D/2D/3D, vec2-4, mat3/4, arrays); trailing `// comment` kept as hint.
- Per-call flow: `setFloat/setSampler2D/...(name, v)` stores the value in the named `ShaderUniform` (shader_common.cpp:1293-1373); `use()` (shader_common.cpp:1190-1208) = optional dynamic recompile, `glUseProgram`, then `applyUniforms()` -> `Shader::applyUniform` (shader.cpp:317-374), a `glUniform*` switch by type; location looked up lazily by name (-1 silently skipped). Values are re-uploaded on every `use()`.
- Rust port: no plumbing needed; three small uniform structs.

| Program | Files (data/shaders) | Uniforms (value; set at) | Notes |
|---|---|---|---|
| shadow | shadow.vert/frag | `tex` sampler2D=0; `shadow_strength`=0.5 (gource.cpp:2146-2147 files, 2307-2308 and 2333-2334 users). Frag: `rgba = (0,0,0, gl_Color.a * tex.a * shadow_strength)`. Modelview translate: files (2,2) (gource.cpp:2150-2154); users `(2,2)*user_scale` (2312) | vertex: `ftransform`, passes `gl_Color`, texcoord0 |
| text | text.vert/frag | `tex`=0; `shadow_strength`=0.7 (hard-coded, 2524); `texel_size`=1/min(512, GL_MAX_TEXTURE_SIZE) (computed once gource.cpp:89-92, set 2525); drawn by `fontmanager.drawBuffer()` (2527) | same vertex stage |
| bloom | bloom.vert/frag | NONE. Vertex: `pos = gl_Vertex.xyz - gl_MultiTexCoord0.yzw` (node centre carried in texcoord yzw), `texcoord0.x` = radius scale. Drawn with `glBlendFunc(GL_ONE,GL_ONE)`, `bloom_shader->use(); bloom_vbo.draw()` (gource.cpp:2200-2207) | varying `vec3 pos`, `gl_Color` |

Matrix and colour are fixed-function state (`ftransform`, `gl_Color`): a port needs one view-proj uniform plus per-vertex colour.

### T2.b Font metrics, atlas, drawing (src/core/fxfont.cpp, gource.cpp)

| Item | Value | Cite |
|---|---|---|
| Font file | FreeSans.ttf (data/fonts); `--font-file` overrides | gource_settings.cpp:35-36,446 |
| FT load | dpi 72, `FT_LOAD_DEFAULT` (hinted), `FT_Set_Char_Size(face,64*size,64*size,dpi,dpi)`, `FT_RENDER_MODE_NORMAL` 8-bit AA | fxfont.h:261; fxfont.cpp:243,81 |
| Advance | integer px: `adv>>6` of hinted metrics; NO kerning; `getWidth` = sum of advances, per byte via FTUnicodeStringItr | fxfont.cpp:87,332-345 |
| Quad | size `(bmp.w+2, bmp.rows+2)`; offset `(bmp.left, -bmp.top)+(0.5,-0.5)` from pen; pen y = baseline | fxfont.cpp:85-86 |
| Heights | glyph height `ceil(metrics.height/64)`; `getMaxHeight` = running max of loaded glyphs; `getHeight()` = `ceil(ascender+descender)` (descender negative) | fxfont.cpp:79,306,324-330,514 |
| Precache | `0-9a-zA-Z;:'",<.>/?-_=+!@#$%^&*()\ ` and space; others lazily (max height can grow) | fxfont.cpp:249 |
| Tab | pen += advance('M')*4 in draw only (not in getWidth) | fxfont.cpp:215,384 |
| Atlas | page = square `min(512, GL_MAX_TEXTURE_SIZE)`, GL_ALPHA 8-bit, CLAMP_TO_EDGE, LINEAR/LINEAR, no mipmaps. Shelf pack: cursor (1,1), pad 3; wrap if `x+w+3>W` -> x=1, y+=max_h+3 (max_h starts 1, never reset per row); new page if `y+rows+3>H`. UVs `u0=(x-0.5)/W, v0=(y-0.5)/H, u1=(x+w+1.5)/W, v1=(y+rows+1.5)/H` | fxfont.cpp:153-195,295; texture.cpp:167-182 |
| draw() order | `align_right: x-=width`; `align_top: y+=getHeight()`; `round: x,y=roundf`; y then = baseline. Defaults: shadow off, shadow_strength 0.7, offset (1,1), round false, align_top true, white | fxfont.cpp:559-580,447 |
| Shadow (FFP path) | extra black pass at +(1,1), alpha = a*0.7, only when `shadow && !fontmanager.use_vbo` (corrected tick 4: condition added); VBO/shader path does the shadow in text.frag | fxfont.cpp:559-579 |
| Shadow (shader buffer) | text.frag: `colour_a=tex(uv).a; shadow_a=tex(uv-texel_size).a*0.7; comb=1-(1-shadow_a)*(1-colour_a); if(comb>0) colour_a/=comb; out=gl_Color*vec4(vec3(colour_a),comb)`; 1px down-right, applies to every glyph in the buffer regardless of dropShadow flag | data/shaders/text.frag |

Font sizes (px at 72dpi). F = font_scale = `viewport_dpi_ratio.x` if >1 else `1 + display.height/1600` (int div), only when default_font_scale (gource.cpp:40-47); `scaled_X = clamp((int)(X*F),1,100)` (gource_settings.cpp:522-527).

| Font | Size | Flags | Cite |
|---|---|---|---|
| fontlarge | 42*F | shadow, round, alignTop | gource.cpp:50 |
| fontmedium | `scaled_font_size` (= clamp(16*F,1,100)) | date/title; dropShadow on, round off (corrected tick 4: size is the scaled setting, not literal 16*F) | :54 |
| fontcaption | 16 (unscaled `caption_size`) | alignTop false | :58 |
| font (HUD) | 14*F | round | :63 |
| fontdirname | `scaled_dirname_font_size` (default 14*F) | alignTop, round forced false at draw (2494, verified) | :67 |
| user name | `scaled_user_font_size` (default 14*F, `--user-font-size`); selected 18 unscaled (corrected tick 4: setting-driven) | alignTop false | user.cpp:305-313 |
| file name | `scaled_filename_font_size` (default 14*F); selected 18 unscaled, colour selection (1,1,0.3) | alignTop true | file.cpp:54-65 |
| key / slider / textbox | 16*F / 16*F / 18*F | | key.cpp:166, slider.cpp:28, gource.cpp:142 |

Text positions (gource.cpp):

| Element | Position | Cite |
|---|---|---|
| Date | x = `width/2 - date_x_offset` (`(int)getWidth*0.5`, refreshed only if changed by >5px), y=20 (top-aligned) | 1470-1472, 2646 |
| Title | alignTop false, `(10, height-10)` baseline | 2652-2656 |
| Logo | "Gource" centred, y = `h/2 - 30*F`; tagline `h/2+10*F`; copyright `h/2+30*F` | 2618-2641 |
| Caption y | `canSeek ? slider.min.y-35 : height - fontmedium.maxH - 20`; with title `min(.., height-20-maxH)`; floor; stacked upward by `fontcaption.getMaxHeight()` until unique | 1778-1846 |
| Caption x | offset 0: `(int)(w/2 - width/2)`; <0: `w + off - width`; >0: off | 1793-1842 |
| Caption alpha | `fade_in=min(2,dur/3)`; `min(1, min(elapsed, max(0,dur-elapsed))/fade_in)`; dur default 10 | caption.cpp:49-53 |
| Pass order | dir names -> unselected users -> unselected files into one buffer, drawn with text shader (shadow 0.7); then selected user and selected file via FFP CPU shadow; date/title/captions/HUD FFP | 2491-2539 |

### T2.c Label placement and culling

| Label | Formula | Cite |
|---|---|---|
| Name fade (all pawns) | `done = nametime - name_interval`; done<1: a=done; 1<done<nametime-1: 1; else nametime-done (corrected tick 4: the middle branch is strict `done>1.0`, so done==1.0 exactly falls to the last branch giving nametime-1=4; port with the same ordering or clamp to 1). nametime 5 (user name_interval init 5.0; file `filename_time`). visible = `!((!selected && name_interval<0) || hidden)` | pawn.cpp:112-127; user.cpp:43; file.cpp:36 |
| User name | anchor world `(pos.x, pos.y - dims.y*0.5)` -> gluProject, y flipped; `x = sx - namewidth*0.5; y = sy - font.getMaxHeight()` (baseline, alignTop false, no rounding). Drawn only if highlight_all/highlighted/selected/alpha>0. alpha = (selected\|\|highlighted\|\|highlight_all) ? user getAlpha : fade. Colour selected=selection, highlighted=highlight_colour, else white | user.cpp:370-395,330 |
| File name | anchor `absPos + (5.5, selected ? -2 : -1)` projected, y flipped; alignTop true. unselected: extension only if `file_extensions` else name; selected: name, 18px, alpha 1. Skip if `!selected && alpha<=0.01` | file.cpp:262-291 |
| File cull | dir in frustum, !hide_filenames, !hide_files | dirnode.cpp:983-990 |
| Dir name skip | root; hide_dirnames; `dir_name_depth>0 && dir_name_depth < depth-1` (root depth 1); `!highlight_dirs && since_last_node_change>5`; not visible | dirnode.cpp:930-943,231-233 |
| Dir name alpha/text | `highlight_dirs ? 1 : max(0,5-since_change)/5`; text = last path component; colour dir_colour; left edge at label_pos (no centring), alignTop true | dirnode.cpp:930-943, spline.cpp:60-69 |
| label_pos | `pos=dir_name_position` (default 0.5, valid 0.1-1.0); `s=0.5-abs(pos-0.5); p=1-s; label = pos1*(p*(1-pos)) + pos2*(p*pos) + spos*s` (pos1 parent projected, pos2 own, spos spline mid); default weights 0.25/0.25/0.5 | spline.cpp; gource_settings.cpp:458,1622 |
| dir_name_depth | default 0 = unlimited; must be >0 if given | gource_settings.cpp:457,1611 |

### T2.d UI geometry (screen px, origin top-left, blend SRC_ALPHA/ONE_MINUS_SRC_ALPHA, no texture on boxes)

| Widget | Geometry / colour / alpha | Cite |
|---|---|---|
| FileKey entry | width `90*F`, height `scaled_font_size+4`, left_margin `scaled_font_size+4`, shadow offset (3,3). Pos x = `alpha*left_margin`, y = pos_y. Shadow quad black, alpha `a*0.333`. Main quad gradient along x: left edge colour `col*0.5`, right edge `col`, alpha a. Ext text at `((int)x+2,(int)y+3)` white, no shadow; count at `((int)x+width+4,(int)y+3)` with shadow. Ext truncated char by char until `getWidth <= width-15*F`, then "..." appended | key.cpp:14-40,115-158 |
| FileKey layout | rebuilt every `update_interval`=1.0 s (gource.cpp:146); sort count desc then ext asc; max visible `max(0,(int)((height-150)/20))`; row y = `n*(scaled_font_size+6)` starting at `scaled_font_size+6`; entry slides src_y->dest_y linearly over 1.0 s (first placement snaps); alpha ramps +/-dt (1 s fade); removed when count<=0 && alpha<=0. Colour = colourHash(ext), white if ext empty | key.cpp:50-110,215-290 |
| Slider | bounds min `(35, height-70)`, max `(width-35, height-35)` drawn as 2px line-strip rectangle; marker 2px vertical line at `x=min.x+(max.x-min.x)*percent` spanning min.y..max.y. Shadow copy offset (2,2), black alpha `0.7*alpha`; main white (slidercol) alpha. alpha ramps +/-dt, hold `fade_time`=1.0 after mouse leaves. Caption (date) at `x=clamp(mouseover - w/2, 1, width-w-1)`, `y = bounds.min.y - 25*F`, font 16*F with shadow | slider.cpp:28-60,90-170 |
| TextBox (hover) | font 18*F; `rect_width = max(getWidth(line)+6)`, `rect_height = 2 + n*(fontsize+4)`; shadow (3,3) black `alpha*0.333`; fill = hovered pawn colour * brightness 0.5 (gource.cpp:143), alpha 1; text white at `((int)x+2, (int)y+3 + i*(fontsize+4))`. setPos(mouse, adjust): `y -= rect_height`; if `x+w>display.w`: `x -= w` when `x-w-fontheight>0` else `x = display.w - w`; if y<0: `y += rect_height + fontheight`; if `y+h>display.h`: `y -= rect_height`. Max 1024 chars/line. Lines: file name then path without leading '/'; or user name | textbox.cpp:20-155; gource.cpp:142-144,2674-2692 |

### T2.e Pawn / file / user colour, size, alpha

| Quantity | Formula | Cite |
|---|---|---|
| Base alpha | `min(elapsed/fadetime,1)`, fadetime 1.0 | pawn.h:93 |
| Quad | centred on pos; w=`size`, h=`size*graphic_ratio` (graphic h/w); UV (0,0)-(1,1); colour `(col, alpha)` | pawn.cpp:154-190 (corrected tick 4: was 150-167) |
| Shadow quad | pos + shadowOffset (2,2) (users `(2,2)*user_scale`); colour `(0,0,0, alpha*0.5)` (gGourceShadowStrength) | pawn.cpp:20,130-152 (corrected tick 4: was 125-148); user.cpp:31 |
| File size | `gGourceFileDiameter*1.05` = 8.0*1.05 | file.cpp:20,29 |
| File colour | selected: white; else lerp(touch_colour, file_colour) with lc=elapsed-last_action while <1 | file.cpp:152-162 |
| File alpha | base; if fade_start>0: `1 - clamp(elapsed-fade_start,0,1)`; fade_start set when idle > file_idle_time | file.cpp:164-205 |
| User size | `20 * user_scale` | user.cpp:29 |
| User colour | `colourHash(name)` or white when a user image is used and !colour_user_images; then `c = c*0.6 + 0.4; c *= 0.9`; selected = white | user.cpp:174-220,333 (corrected tick 4: the `*0.6+0.4` / `*0.9` tint is at user.cpp:219-220, after assignUserImage's image selection) |
| User alpha | base; after `user_idle_time` (3.0) idle: `1 - min(elapsed-last_action-idle, 1)` | user.cpp:343-349 |
| Name colour | user: selected -> selection (1,1,0.3), highlighted -> highlight_colour, else `namecol` (white, pawn.cpp:41); file: selected -> selection; unselected file text is drawn with `file_font` colour = `filename_colour` setting (file.cpp:54-65), not hard white (corrected tick 4: `namecol` is white but the file name draw ignores it) | user.cpp:330; file.cpp:141 |

### T2.f Display / DPI / frameless mapping

- SDL2 window flags: OPENGL|SHOWN; `frameless` -> BORDERLESS; `resizable && !frameless` -> RESIZABLE; fullscreen -> FULLSCREEN; ALLOW_HIGHDPI only if `--high-dpi` and not Windows (display.cpp:82-92). Default `high_dpi_aware=false` (display.cpp:47), so macOS gets a 1x backbuffer unless `--high-dpi`.
- Windows branch: with `--high-dpi`, call `SetProcessDpiAwareness(PER_MONITOR=2)` (SHCORE, 8.1+) else `SetProcessDPIAware()` before window creation (display.cpp:549-593), needed to get real 4K pixels. winit is DPI-aware by default: request `PhysicalSize` and ignore scale factor unless `--high-dpi`.
- Viewport: on resize `width/height = SDL_GL_GetDrawableSize` (pixels), `glViewport(0,0,w,h)`; `viewport_dpi_ratio = drawable/window size` (display.cpp:173-181,520-545). Only consumer: font_scale (T2.b). Port: drawable = `inner_size()` physical px; ratio = scale_factor only when high-dpi requested, else 1.
- Frameless toggle (windowed only, display.cpp:405-470): SetWindowBordered(false), restore size/position (SDL bug 2791 workaround), then recreate video mode to drop RESIZABLE; Windows also uses GetWindowRect so the borderless window keeps outer framed size. winit: `set_decorations(false)`.
- Fullscreen toggle uses FULLSCREEN_DESKTOP, saves/restores windowed size (display.cpp:360-400). Export path bypasses all of this (offscreen target).
- Clear colour (0,0,0,1) default; `enable_alpha` makes clear alpha 0 (display.cpp:40-62).

### T2.g Textures (src/core/texture.cpp, src/user.cpp)

| Item | Detail | Cite |
|---|---|---|
| Loader | SDL_image `IMG_Load`; 8-bit palette converted to RGBA32; only 3/4 bytes-per-pixel accepted (RGB/RGBA/BGR/BGRA by mask), others throw TextureException | texture.cpp:258-305 |
| Upload | internal format RGBA (ALPHA/LUMINANCE only for fonts); mipmaps via `gluBuild2DMipmaps` (GLU rescales non-POT to POT) else `glTexImage2D`; straight alpha | texture.cpp:216-236 |
| Filtering | mipmapped: min LINEAR_MIPMAP_LINEAR (trilinear default true, texture.cpp:37), mag LINEAR; else LINEAR/LINEAR. Wrap CLAMP_TO_EDGE for file/user textures; beam.png default REPEAT | texture.cpp:167-182; gource.cpp:36,77-78 |
| Default images | file.png 256x256 RGBA; user.png 384x512 RGBA (graphic_ratio 4/3); beam.png 128x1 RGBA; bloom.tga 512x512 RGB RLE; bloom_alpha.tga 512x512 RGBA RLE | gource.cpp:36,76-78 |
| User image lookup | `--user-image-dir`: scans `.jpg/.jpeg/.png` (ext case-insensitive); key = filename without ext (macOS NFC-normalised); exact author-name match; image colour not tinted (usercol white) unless `--colour-images`. Missing -> `--default-user-image`, else data `user.png`. Loaded with `grabFile(mipmap=true, CLAMP_TO_EDGE)`; aspect via graphic_ratio | gource_settings.cpp:800-860; user.cpp:183-217 |
| Logo / background | `grabFile(logo, mipmap_logo)`, `grabFile(background_image)` | gource.cpp:99,104 |

Port notes: use `image` crate plus own mip chain and trilinear clamp-to-edge sampler; exact GLU POT resampling is a sharpness-only difference.

### T2 gaps
- GLU rounding rule for 384-wide user.png not verified; Bounds2D line width on HiDPI; where `--high-dpi` calls `enableHighDPIAwareness` not traced; file_scale effect on file size not re-derived.

## Verification (tick 4)

Sampled 36 claims against source: shader regex/use()/applyUniform and every uniform value and drawing site (gource.cpp 2146-2334, 2200-2207, 2522-2528, text.frag), FT load/advance/quad/atlas pack and UV formulas, draw() order, tab, precache, font construction and scaled sizes, text positions (date/title/logo/caption), name-fade, user/file label anchors, dir-name skip/alpha/label_pos (dirnode.cpp, spline.cpp:63-69), FileKey/Slider/TextBox geometry, pawn/file/user colour+alpha, texture loader/upload/filter and default image names, user-image lookup. Most were exact (shader and atlas values, UI geometry, label formulas all confirmed). Corrections made inline above: shifted pawn.cpp shadow/quad lines, user-colour tint line range, file-name colour (filename_colour, not white), setting-driven font sizes, FFP shadow condition, name-fade boundary case at done==1.0.
Not re-verified: Windows DPI branch (display.cpp:549-593), frameless toggle details, caption stacking (gource.cpp:1778-1846), GLU resampling. Confirmed atlas `max_glyph_height` initialised once per page (fxfont.cpp:139) and never reset per row.
