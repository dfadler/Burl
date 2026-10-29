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
| 3D scene | `gluPerspective(fov=90, w/h, znear=0.1, zfar=max+1)` + `gluLookAt(pos,target,up)`; start z=-300 | src/zoomcamera.cpp:23-26, 54, 75-82; src/gource.cpp:138 |
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
