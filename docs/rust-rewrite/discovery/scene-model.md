# Gource scene / simulation model (RDirNode, RFile, RUser, Pawn, RAction)

All paths relative to repo root. Line cites are `file:line` in `src/` unless noted. "f32"/"f64" mark the C++ arithmetic type that matters for bit-exact porting. Drawing code is listed only where it affects simulation state or defines "visually equivalent" output.

## 0. Frame pipeline (how gource.cpp drives the model)

`Gource::update` (gource.cpp:247-269): `scaled_dt = min(dt, max_tick_rate)` where `max_tick_rate = 1.0/60.0` (gource.cpp:156; f64 -> f32). With a frame exporter, `scaled_dt = max_tick_rate` fixed (gource.cpp:251-253). Then `scaled_dt *= time_scale` (default 1.0, gource_settings.cpp:413). `runtime += scaled_dt` unless paused (gource.cpp:259). `logic(runtime, scaled_dt)`: `t` = runtime (used for action lag), `dt` = scaled_dt. **The sim depends on wall-clock dt unless exporting; a deterministic Rust port needs an injectable fixed dt.**

`Gource::logic` order (gource.cpp:1568-1878), non-paused:
1. Rotate if `rotate_angle != 0` (gource.cpp:1655-1685): `root->rotate(s,c)` (or about camera centre if manual) and every user `setPos(rotate_vec2(pos,s,c))`. `rotate_vec2(v,s,c) = (v.x*c - v.y*s, v.x*s + v.y*c)` (core/vectors.cpp:3). `rotate_angle` is set by camera auto-rotate (gource.cpp:1525-1546: step `min(remaining, 90*angle_rate)` deg->rad, `angle_rate = max(dt, 1-|rem/90-0.5|*2)*dt`; triggers when `dir_bounds.area()>10000` and bounds aspect `<0.67` -> remaining=90deg) or by mouse (gource.cpp:369-373). Rotation touches `pos` and `spos` of every dir (dirnode.cpp:116-135), not `accel`/file offsets.
2. `recolour` flag -> `changeColours()` (gource.cpp:1552-1566): `gStringHashSeed = rand()%10000 + 1`, then `colourize()` every user/file, `file_key.colourize()`.
3. Read log, advance `currtime` (gource.cpp:1723-1733): `time_inc = dt*86400.0*days_per_second` (stored f32), `seconds=(int)time_inc`, `subseconds += time_inc - seconds` (f32), carry when `>=1.0`.
4. **Delete expired files**: for each in `gGourceRemovedFiles` -> `Gource::deleteFile` (gource.cpp:962-988), then clear (gource.cpp:1736-1740).
5. Process commits with `timestamp <= currtime` -> `processCommit(commit, t)` -> creates files/users/actions (section 6).
6. `updateBounds()` (gource.cpp:1335-1362), `interactUsers()` (1292-1333), `updateUsers(t,dt)` (1365-1416), `interactDirs()` (1418-1449), `updateDirs(dt)` (1451-1454: `root->applyForces(*dirNodeTree); root->logic(dt);`), `updateCamera(dt)`, `updateTime`.

**Container iteration orders (affect float accumulation order and `rand()` call order):**

| Container | Type | Order | Source |
|---|---|---|---|
| users | `std::map<string,RUser*>` | sorted by username (byte-wise `std::string <`) | used gource.cpp:1314,1321,1371 |
| files | `std::map<string,RFile*>` | sorted by fullpath | gource.cpp:1007 |
| `gGourceDirMap` | `std::map<string,RDirNode*>` | sorted by abspath (trailing '/') | dirnode.cpp:32,84 |
| `RDirNode::children`, `files` | `std::list` | insertion order (push_back); erase preserves order | dirnode.cpp:261,410 |
| `gGourceRemovedFiles` | `std::vector<RFile*>` | expiry order (dir-tree DFS order within a frame); touch-after-expire erases the entry | file.cpp:219,240 |
| `RUser::actions` / `activeActions` | `std::list<RAction*>` | FIFO | user.cpp:57,251,264 |

## 1. Shared math helpers

| Item | Definition | Source |
|---|---|---|
| `PI` | `#define PI 3.14159265` (f64 literal, 9 sig. digits, NOT std PI) | core/pi.h:31 |
| `normalise(v)` | `l=length(v); l>0 ? v/l : v` (zero vector stays zero) | core/vectors.cpp:7-23 |
| `stringHash(s)` | `val=0; for i<n: val += s[i] * (gStringHashSeed ^ (n-i))`; **`^` is XOR not power**; `s[i]` is signed `char` (UTF-8 bytes >=128 negative); `int` arithmetic, signed overflow wraps in practice; `if(val<0) val=-val`; `n=str.size()` | core/stringhash.cpp:32-46 |
| `gStringHashSeed` | int, default 31 (re-set to 31 at settings load gource_settings.cpp:489; changed by recolour) | core/stringhash.cpp:30 |
| `vec2Hash(s)` | `h=stringHash; x=((h/7)%255)-127; y=((h/3)%255)-127; normalise(vec2(x,y))` (ints, then to f32) | stringhash.cpp:48-57 |
| `colourHash(s)` | `h=stringHash; if(h==0) h++; r=(h/7)%255; g=(h/3)%255; b=h%255; normalise(vec3(r,g,b))` | stringhash.cpp:71-83 |
| `Bounds2D::overlaps` | inclusive: false iff `max.y<b.min.y \|\| min.y>b.max.y \|\| max.x<b.min.x \|\| min.x>b.max.x` | core/bounds.h:117-125 |
| `Bounds2D::contains(p)` | false if `first` (empty); else inclusive box test | core/bounds.h:108-115 |
| `Bounds2D::reset` | min=max=(0,0), first=true; `update` expands (first sample initialises) | core/bounds.h:54-106 |
| `rand()` | libc `rand()`, never seeded except `srand(time(0))` when random start position is requested (gource_settings.cpp:1319). Unseeded = default seed 1 -> libc-specific sequence. Used only in the 3 overlap-resolve spots + recolour (section 9) | - |

## 2. Pawn (base of RFile and RUser) - pawn.h / pawn.cpp

| Field | Type | Default (ctor pawn.cpp:22-46) |
|---|---|---|
| pos | vec2 | ctor arg |
| shadowOffset | vec2 | (2,2) |
| name | string | ctor arg |
| namewidth | f32 | 0 |
| accel | vec2 | zero-initialised: GLM_FORCE_CTOR_INIT (core/vectors.h:34) makes vec ctors zero-init (corrected tick 3) |
| speed | f32 | 1.0 |
| elapsed | f32 | 0.0 (per-pawn clock, advanced in `logic`) |
| fadetime | f32 | 1.0 |
| nametime | f32 | 5.0 |
| name_interval | f32 | 0.0 |
| namecol | vec3 | (1,1,1) |
| shadow | bool | false |
| hidden | bool | false |
| tagid | int | ctor arg |
| selected, mouseover | bool | false |
| size | f32 | 0.0 |
| graphic_ratio | f32 | 1.0 (set by `setGraphic`: `h/(float)w`, pawn.cpp:82-92) |
| graphic, dims | TextureResource*, vec2 | 0; dims=(size,size*ratio) at `setGraphic` time |

Methods: `logic(dt)`: `elapsed += dt; if(!hidden && name_interval>0) name_interval -= dt` (pawn.cpp:74-80). `getAlpha() = min(elapsed/fadetime, 1.0)` (pawn.h:93; fade-in over 1 s). `showName(): if(name_interval<=0) name_interval = nametime` (pawn.cpp:60). `updateQuadItemBounds`: half=`size*0.5`; box `pos -/+ (half, half*graphic_ratio)` (pawn.cpp:64-72). `nameVisible = !((!selected && name_interval<0) || hidden)` (pawn.cpp:112). `drawName` fade envelope: `done=nametime-name_interval; done<1 -> alpha=done; 1<done<nametime-1 -> 1; else nametime-done` (pawn.cpp:116-128). Shadow: offset quad coloured `(0,0,0,alpha*gGourceShadowStrength)`, `gGourceShadowStrength=0.5` (pawn.cpp:20,140). Draw: quad of `size x size*ratio` centred at pos, colour `getColour()`, alpha `getAlpha()` (pawn.cpp:161-193).

Image aspect (feeds user bounds, hence the user quadtree): `data/user.png` is 384x512 -> `graphic_ratio = 1.3333`; `data/file.png` is 256x256 -> 1.0 (loaded at gource.cpp:35-36). User image overrides change the ratio.

## 3. RDirNode (dirnode.h/.cpp)

### 3.1 Fields (dirnode.h:38-100; constructor dirnode.cpp:34-70)

| Field | Type | Initial |
|---|---|---|
| abspath | string | arg + trailing '/' if missing (dirnode.cpp:78-80); registered in `gGourceDirMap` |
| path_token / path_token_offset | string / int | set by `adjustPath` (dirnode.cpp:200-217) |
| parent | RDirNode* | **always 0 after ctor** (ctor assigns the shadowed parameter: `parent = 0; setParent(parent)`, dirnode.cpp:38-39, note :43). Real parent is set only by `addNode` -> `setParent` |
| children / files | list | empty |
| spline | SplineEdge | render only |
| col | vec4 | `calcColour()` at ctor |
| pos | vec2 | (0,0) (dirnode.cpp:46-53; the parent branch is dead code) |
| spos | vec2 | (0,0) |
| vel | vec2 | (0,0) (unused afterwards) |
| accel, prev_accel | vec2 | (0,0) |
| dir_area | f32 | `calcRadius` |
| visible | bool | false |
| in_frustum | bool | uninitialised (render only) |
| position_initialized | bool | false |
| since_node_visible, since_last_file_change, since_last_node_change | f32 | 0.0 |
| file_area | f32 | `(float)(r*r*PI)`, `r = gGourceFileDiameter*0.5` (=4.0) -> 16*3.14159265 = 50.2654824 (dirnode.cpp:55-57) |
| dir_radius, parent_radius | f32 | `calcRadius` |
| depth | int | 1 for root; `parent.depth+1` (dirnode.cpp:228-239) |
| visible_count | int | 0 |
| quadItemBounds, node_count | inherited QuadItem | Bounds2D empty (reset); `node_count` uninit until inserted in a tree |

Constants (dirnode.cpp:20-26): `gGourceMinDirSize=15.0` (unused), `gGourceForceGravity=10.0`, `gGourceDirPadding=1.5`, `gGourceGravity=true` (flag unused in force code), `gGourceFileDiameter=8.0` (file.cpp:20).

### 3.2 Path/tree structure (the "split / merge" logic)

The tree is a compressed trie of directory paths: nodes exist only for directories that directly contain a file or that are the longest common '/'-terminated prefix of >=2 children. Paths always start and end with '/'. Gource keeps a `root` pointer; initially `new RDirNode(0,"/")` (gource.cpp:924).

`prefixedBy(path)` (dirnode.cpp:142-149): empty -> false; if `path` doesn't end in '/' append '/'; true iff `abspath.find(path)==0` (abspath starts with path; includes equality).

`commonPathPrefix(str)` (dirnode.cpp:362-375): scan while `abspath[c]==str[c]`, remember index of last '/' within the matched run; return `str.substr(0,slash+1)` or "" if none.

`RDirNode::addFile(f)` returns bool; `f->path` = dir part of file (trailing '/'), `f->fullpath` = full name (file.cpp:112-134). Exact algorithm (dirnode.cpp:377-496):
1. **Non-matching path** (`f->path` does not start with `abspath`): if `parent!=0` return false. Else (root): `common = commonPathPrefix(f->path)`, `""->"/"`; `newparent = new RDirNode(0, common)`; `newparent->addNode(this)`; `return newparent->addFile(f)`. Caller (`Gource::addFile`) walks `root = root->getParent()` while non-null (gource.cpp:1013-1016).
2. **Root rename**: `parent==0 && abspath=="/" && f->path != abspath && noFiles() && noDirs()` -> `changePath(f->path)` (erases old gGourceDirMap key, registers new) (dirnode.cpp:400-404).
3. **Exact dir**: `f->path==abspath` -> `files.push_back(f); if(!f->isHidden()) visible_count++` (new files are hidden, so no increment); `f->setDir(this)`; `fileUpdated(false)`; return true.
4. Else try each child in order: `added = child->addFile(f); if(added) break;`.
5. `if(added && parent!=0) return true;`
6. **File-becomes-directory**: for each file `g` in this->files in order: if `f->path.find(g->fullpath)==0` then `g->remove()` (forced removal, fade out) and break (runs for the root even if added in step 4).
7. `if(added) return true;`
8. **New leaf dir**: `node = new RDirNode(this, f->path); node->addFile(f); addNode(node);`
9. **Split to create intermediate dir**: iterate `children` in order; for child c: `common = c->commonPathPrefix(f->path)`; first c with `common.size() > abspath.size() && common != f->path` -> `commonpath=common, commonPos=c->pos`, break. If found: `cnode = new RDirNode(this, commonpath); cnode->setPos(commonPos)`; move every child with `prefixedBy(commonpath)` (list order) out of `children` into `cnode->addNode(child)`; finally `addNode(cnode)`. (cnode's `setPos` is overwritten at its first `move()` by `setInitialPosition` because `position_initialized==false` and cnode has children so `empty()` is false; dirnode.cpp:807-809.)

`addNode(node)` (dirnode.cpp:241-267): for each current child (in order) with `child->prefixedBy(node->path)`: erase from children and `node->addNode(child)` (recursive); then `children.push_back(node); node->setParent(this); nodeUpdated(false)`.

`setParent`: no-op if same non-null parent; sets `parent`, `adjustPath()` (path_token = `abspath.substr(parent.path_token_offset, size - parent_offset - 1)`, i.e. the segment without trailing '/'; offset = abspath.size()), `adjustDepth()` (recurses to children) (dirnode.cpp:219-239).

**Removal / merging** `removeFile(f)` (dirnode.cpp:275-320): false if `f->path` doesn't start with `abspath`. If `f->path==abspath`: find f in files, erase, `if(!f->isHidden()) visible_count--`, `fileUpdated(false)`, true (false if not found). Else for each child in order: if `child->removeFile(f)`: if child `noFiles() && noDirs()` -> erase child from list, delete it (unregisters from gGourceDirMap), `nodeUpdated(false)`; return true. Empty dirs are reaped bottom-up (each ancestor checks its own immediate child on unwind). **Intermediate (split) dirs are NOT merged back when they fall to one child.** The root is never deleted.

`isDir(path)` (dirnode.cpp:160-171): `prefixedBy(path)` true; else false unless `path` starts with `abspath`; else any child `isDir`. `findDirs(path,out)` (dirnode.cpp:174-184): if `prefixedBy(path)` push self and stop descending; else recurse children. `getFilesRecursive` inserts this dir's files at the *front* of the output list then recurses children, so later dirs' files precede earlier ones (dirnode.cpp:186-194). `Gource::addFile` rejects a file whose path+"/" is already a directory (`root->isDir`, gource.cpp:998-1001) and when `max_files>0 && files.size()>=max_files` (gource.cpp:995).

### 3.3 Update propagation, visibility

- `fileUpdated(userInitiated)` (dirnode.cpp:99-105): `calcRadius(); since_last_file_change=0; nodeUpdated(userInitiated)`.
- `nodeUpdated(userInitiated)` (dirnode.cpp:107-114): `if(userInitiated) since_last_node_change=0; calcRadius(); updateFilePositions(); if(visible && noDirs() && noFiles()) visible=false; if(parent) parent->nodeUpdated(true)`. Ancestors always get `userInitiated=true`.
- `addVisible()` (called when a hidden file is first un-hidden): `visible_count++; visible=true` (dirnode.cpp:331-334).
- `isVisible()` (dirnode.cpp:336-348): true if `visible`; else if any child `isVisible()` then **sets `visible=true`** and returns true (mutating DFS).
- `empty()` = `visible_count==0 && noDirs()` (dirnode.cpp:643). Empty dirs are excluded from the force quadtree (gource.cpp:1443) and skip initial placement.

### 3.4 Radius and file layout

`calcRadius` (dirnode.cpp:574-594), f32 unless noted:
```
total_file_area = file_area * visible_count
dir_area        = total_file_area + sum(child.dir_area)            // child order
dir_radius      = max(1.0f, (float)sqrt(dir_area)) * gGourceDirPadding   // 1.5
parent_radius   = max(1.0f, (float)sqrt(total_file_area) * gGourceDirPadding)
```
(`dir_radius_sqrt` is never assigned.) Children's `dir_area` is whatever was last computed (stale until the child's own update; child updates precede parent's in the `nodeUpdated` chain).

`updateFilePositions` (dirnode.cpp:842-880) - concentric rings, files in `files` list order, skipping hidden (they get dest=(0,0), distance=0):
```
max_files=1; diameter=1; file_no=0; d=0.0f; files_left=visible_count
for f in files: if hidden {dest=0; dist=0; continue}
   f.dest = calcFileDest(max_files,file_no); f.distance = d
   files_left--; file_no++
   if file_no >= max_files:
        diameter++; d += gGourceFileDiameter (8.0)
        max_files = (int)max(1.0, diameter*PI)      // f64: ring slots 1,6,9,12,15,18,...
        if files_left < max_files: max_files = files_left
        file_no = 0
```
`calcFileDest(max_files,file_no)` (dirnode.cpp:831-840): `arc = 1.0/(float)max_files` (f64), `frac = arc*0.5 + arc*file_no` (f64), `dest = (sinf(frac*PI*2.0), cosf(frac*PI*2.0))` (argument computed in f64, rounded to f32 for sinf/cosf). First file: ring 0, distance 0 -> at dir centre.

### 3.5 Force model (dirnode.cpp:604-714)

`applyForceDir(node)` (pairwise repulsion when circles overlap):
```
dir = node.pos - pos;  posd2 = length2(dir);  sum = r_me + r_node   // dir_radius
if (posd2 - sum*sum) > 0: return              // not overlapping
posd = sqrt(posd2);  distance = posd - r_me - r_node        // <= 0
if posd < 0.00001: accel += normalise(vec2(rand()%100 - 50, rand()%100 - 50)); return
accel += distance * normalise(dir)             // away from node (distance negative)
```
`applyForces(quadtree)` per dir: children recursed FIRST, depth-first in child-list order; then self (dirnode.cpp:647-714). Root returns before its own forces. A non-root dir does these steps in this order (float add order matters):
1. `quadtree.visitItemsInBounds(quadItemBounds, DirForceFunctor)` (3.7) calling `applyForceDir(d)`.
2. `applyForceDir(parent)` always.
3. Gravity: `parent_dist = |parent.pos - pos| - (dir_radius + parent.parent_radius)`; `accel += 10.0 * parent_dist * normalise(parent.pos - pos)` (`gGourceForceGravity`).
4. If `pparent = parent.parent` exists: `edge_n = normalise(parent.pos - pparent.pos)`; `accel += (parent.pos + (parent.radius + radius)*edge_n) - pos`.
5. Sibling repulsion: `visible=1; sib_accel=(0,0)`; for each sibling `n != this` with `n.isVisible()`: `visible++; sib_accel -= normalise(n.pos - pos)`. If `visible>1`: `slice = (float)((parent.radius * PI) / (float)(visible+1))` (f64 divide), `accel += sib_accel * slice`.

`move(dt)` (dirnode.cpp:799-825): root returns (never moves, accel unused). If `!empty() && !position_initialized` -> `setInitialPosition()`. Then `pos += accel*dt`. If `elasticity>0` (default 0.0): `diff=accel-prev_accel; m=dt*elasticity; accel3 = prev_accel*(1-m) + diff*m; pos += accel3; prev_accel=accel3`. Finally `accel=(0,0)`. **No velocity, no damping term**: a first-order integrator `pos += accel*dt`. Non-root dirs move even while empty/invisible.

`setInitialPosition` (dirnode.cpp:780-797): `pos = parent.pos`; if grandparent exists `pos += normalise(normalise(parent.pos - gp.pos)*2.0f + vec2Hash(abspath))` else `pos += vec2Hash(abspath)`; `spos = pos - (parent.pos - pos)*0.5f`; `position_initialized=true`. Deterministic, no rand.

`updateSplinePoint(dt)` (dirnode.cpp:762-778; sim state `spos`, used for edge render): root skips. `td=(parent.pos-pos)*0.5f; mid=pos+td; delta=mid-spos; if length2(delta)>length2(td): spos += normalise(delta)*(length(delta)-length(td)); spos += delta*min(1.0f, dt*2.0f)` (`delta` not recomputed after the clamp).

`logic(dt)` (dirnode.cpp:895-928): `move; updateSplinePoint; if(parent) node_normal=normalise(pos-parent.pos); for f in files: f->logic(dt); for child: child->logic(dt); calcColour(); if(visible) since_node_visible+=dt; since_last_file_change+=dt; since_last_node_change+=dt`. (Pre-order: dir, its files, then children.)

### 3.6 Colour (dirnode.cpp:545-568)
```
brightness = max(0.6f, 1.0f - min(1.0f, since_last_node_change/3.0f))
col = vec4(brightness,brightness,brightness,1.0)
for f in files (skip hidden): col += vec4(f.getColour()*brightness, f.getAlpha())
col /= (float)fcount + 1.0        // f64 sum, converted for division
```
Bloom colour = `col * bloom_intensity` (default 0.75), radius `dir_radius*2.0*bloom_multiplier` (default 1.0), drawn only if `in_frustum && isVisible()` (dirnode.cpp:1056-1069, 1150-1180). `averageFileColour` averages visible-file colours then children (dirnode.cpp:510-539).

### 3.7 Quadtree use (dirs)

Rebuilt every frame (`Gource::interactDirs`, gource.cpp:1418-1449): bounds = `dir_bounds` +/-1; `max_depth = 1`, or `gGourceMaxQuadTreeDepth=6` (gource.cpp:23) if `dir_bounds.area() > 10000.0`; `QuadTree(bounds, max_depth, max_node_items=1)`; items inserted in `gGourceDirMap` order for dirs with `!empty()`.
`dir_bounds` (gource.cpp:1352-1361) is the union of `quadItemBounds` of **visible** dirs; `updateQuadItemBounds` runs only for visible dirs: box `pos +/- (dir_radius, dir_radius)` (dirnode.cpp:1182-1189). Non-visible non-empty dirs keep stale/empty bounds.
Quadtree semantics needed to reproduce the visit set (core/quadtree.cpp): root depth = 1 (ctor `parent_depth+1`, :302-307); `allowMoreItems = children.empty() && (depth >= max_depth || items.size() < max_items)` (:38); `addItem` stores in this leaf if allowed (item->node_count++), else if children exist `addToChild`, else split into 4 (TL, TR, BL, BR at `bounds.centre()`), re-insert old items in order, then add the new item; `addToChild` adds to **every** child whose bounds `overlaps` the item bounds (inclusive) (:96-104). `visitItemsInBounds(b)`: if the node has items visit **all** of them (no per-item test); else for children i=0..3 with `!child.empty() && b.overlaps(child.bounds)` recurse (:248-264). With `max_depth==1` the root never splits => every dir interacts with every other, in dirmap order.
`DirForceFunctor` (dirnode.h:212-245) skips: self, `this.parent`, any `d` whose parent is this, any ancestor of this (`this->isParent(d)`), any descendant (`d->isParent(this)`); items in more than one leaf (`node_count != 1`) are de-duplicated with a per-query `std::set` (first visit wins).

## 4. RFile (file.h/.cpp) : Pawn

| Field | Type | Init (file.cpp:27-71) |
|---|---|---|
| hidden (Pawn) | bool | **true** until first `touch` |
| size | f32 | `(float)(gGourceFileDiameter*1.05)` = 8.4 |
| radius | f32 | `size*0.5` (unused) |
| speed | f32 | 5.0 |
| nametime / name_interval | f32 | `filename_time` (default 4.0, min 2.0 validated gource_settings.cpp:945-947) |
| namecol | vec3 | (1,1,1) |
| file_colour | vec3 | ctor arg (`cf.colour`) |
| touch_colour | vec3 | zero-initialised via GLM_FORCE_CTOR_INIT (core/vectors.h:34) until first touch |
| dir | RDirNode* | 0 until `addFile` calls `setDir` |
| removed_timestamp | time_t | 0 |
| forced_removal, expired, removing | bool | false |
| fade_start | f32 | -1.0 |
| last_action | f32 | 0.0 |
| dest, distance | vec2, f32 | dest uninit; distance 0 (set by dir layout, 3.4) |
| shadow | bool | true |
| path, fullpath, ext | string | from `setFilename` |

`setFilename(abs)` (file.cpp:112-134): `fullpath=abs`; `pos=fullpath.rfind('/')`; if found `path=name.substr(0,pos+1); name=name.substr(pos+1)` (name==abs at this point); else `path=""`, `name=abs`. `ext`: `dotsep=name.rfind('.')`; if found and not last char -> `ext=name.substr(dotsep+1)`; else if `file_extension_fallback` (default false, gource_settings.cpp:504) `ext=name`; else empty. (Differs from `RCommit::fileColour`, below.)

Lifecycle:
- **Create**: `Gource::addFile` (gource.cpp:991-1019): `tagid=tag_seq++`; `new RFile(cf.filename, cf.colour, vec2(0,0), tagid)`; `files[filename]=file`; `root->addFile(file)`; `file_key.inc`. Starts hidden, not counted in `visible_count`/`dir_area`.
- **Touch** `touch(ts, colour)` (file.cpp:227-252): return if `forced_removal || (removing && ts < removed_timestamp)`. Else `fade_start=-1; removing=false; removed_timestamp=0; last_action=elapsed; touch_colour=colour;` un-expire (erase from `gGourceRemovedFiles` if `expired`; `expired=false`); `showName(); setHidden(false); dir->fileUpdated(true)`. `RFile::setHidden(false)` on a hidden file with a dir calls `dir->addVisible()` first (file.cpp:254-260).
- **Per-frame** `logic(dt)` (file.cpp:175-225): `Pawn::logic(dt)`; `accel = dest*distance - pos; a2 = accel*speed*dt; if length2(a2) > length2(accel) a2 = accel; pos += a2` (pos is the offset relative to the dir; exponential approach, rate 5/s, clamped). Idle fade: `if(fade_start<0 && file_idle_time>0 && elapsed-last_action > file_idle_time) fade_start=elapsed` (`file_idle_time` default 0 = never; `file_idle_time_at_end` replaces it when the log is finished/stop reached, gource.cpp:1180-1182). Expiry: `if(fade_start>0 && !expired && elapsed-fade_start >= 1.0)`: `expired=true`, push to `gGourceRemovedFiles` if absent. Finally `if(isHidden() && !forced_removal) elapsed = 0.0` (hidden files never age).
  Quirk: fade tests use `fade_start > 0.0f`, so `fade_start == 0.0` means "not fading".
- **Remove** `remove(ts)` (file.cpp:76-81): `last_action=fade_start=elapsed; removing=true; removed_timestamp=ts`. `remove()` forced (file-became-dir): `forced_removal=true; remove(0)` (touch then ignored forever).
- **Alpha** `getAlpha()` (file.cpp:164-173): `a = min(elapsed/1.0, 1)` (fade-in); `if(fade_start > 0) a = 1 - clamp(elapsed-fade_start, 0, 1)` (fade-out overrides).
- **Colour** `getColour()` (file.cpp:152-162): selected -> white; `lc=elapsed-last_action; if lc<1: touch_colour*(1-lc) + file_colour*lc` else `file_colour`.
- **Delete**: at the start of next frame, `Gource::deleteFile` (gource.cpp:962-988): `root->removeFile(file)` (3.2); clear hover/selection; every user `fileRemoved(file)` (drops queued and active actions targeting it, user.cpp:62-86); `files.erase(fullpath)`; `file_key.dec`; `delete`.
- `getAbsolutePos() = pos + dir->pos` (file.cpp:96). `overlaps(p)`: box `abs +/- (size/2, size/2*ratio)` inclusive.
- `colourize()` (recolour key only): `file_colour = ext.size() ? colourHash(ext) : (1,1,1)` (file.cpp:136-138).

Initial `file_colour` comes from the commit parser: `RCommit::fileColour(filename)` (formats/commitlog.cpp:308-319): `dot=rfind('.'), slash=rfind('/')`; if `dot!=npos && dot+1<size && (slash==npos || slash<dot)` -> `colourHash(filename.substr(dot+1))` else `(1,1,1)`. Custom log format may supply an explicit colour (formats/custom.cpp:86-91). `RCommitFile` ctor prepends '/' when missing (commitlog.cpp:295-297). `ModifyAction` later overwrites `file_colour` with the commit's `cf.colour`.

## 5. RAction (action.h/.cpp)

| Field | Type | Init |
|---|---|---|
| colour | vec3 | per subclass |
| source / target | RUser* / RFile* | ctor |
| timestamp | time_t | commit timestamp |
| t | f32 | sim `runtime` at creation (lag test) |
| progress | f32 | 0.0 |
| rate | f32 | 0.5 (set to 2.0 by user lag catch-up, user.cpp:250) |

| Type | colour | extra |
|---|---|---|
| CreateAction (cf.action=="A") | (0,1,0) action.cpp:102 | - |
| RemoveAction ("D") | (1,0,0) action.cpp:106 | on finish `target->remove(timestamp)` (action.cpp:109-117; fires on the frame `old<1 && new>=1`) |
| ModifyAction (anything else) | (1.0,0.7,0.3) action.cpp:120 | `modify_colour = cf.colour`; `apply()` = touch then `target->setFileColour(modify_colour)` (action.cpp:123-126) |

`logic(dt)` (action.cpp:28-38): return if `progress>=1`; **if `progress==0.0` call `apply()`** (= `target->touch(timestamp, colour)`); `action_rate = min(10.0f, rate * max(1.0f, (float)source->getPendingActionCount()))`; `progress = min(progress + action_rate*dt, 1.0f)`. Nominal duration 1/0.5 = 2 s with <=1 pending; shorter with a backlog (pending = queued count, not active). `isFinished = progress >= 1.0`. Beam render: quad from source pos to file abs pos, `perp=(-n.y,n.x)`, `offset = perp*target.size*0.5`, `offset_src = offset*0.3`; alpha `1-progress` at file end, `alpha*0.1` at user end (action.cpp:40-99).

## 6. Commit -> actions (gource.cpp:1188-1290)

`processCommit(commit,t)`: for each file entry `cf` in order:
- filename ends with '/': only `action=="D"` is used (others skipped): `root->findDirs(filename, dirs)`; for each dir, `getFilesRecursive` then `addFileAction(commit, cf, file, t)` per file (a Remove action each).
- else: look up `files[cf.filename]`; if absent `addFile(cf)` **even when action is "D"**; if `addFile` returns 0 (max_files / path is a dir) skip.
`addFileAction`: find/create user (`addUser`); new user highlighted if `highlight_all_users` or name in `highlight_users`; `commit_seq++`; create Remove ("D") / Create ("A") / Modify (else); `user->addAction(a)`.

## 7. RUser (user.h/.cpp) : Pawn

| Field | Type | Init (user.cpp:24-48) |
|---|---|---|
| pos | vec2 | spawn: `dir_bounds.centre()` if `dir_bounds.area()>0` else (0,0) (gource.cpp:1023-1029); all users spawned in one frame share the point |
| speed | f32 | `max_user_speed` (default 500) |
| size | f32 | `20.0*user_scale` (default 1.0) |
| shadowOffset | vec2 | `(2,2)*user_scale` |
| shadow | bool | true |
| highlighted | bool | false |
| actions, activeActions | list<RAction*> | empty; `actionCount`, `activeCount` size_t = 0 |
| last_action | f32 | 0.0 |
| action_interval | f32 | 0.2 |
| name_interval | f32 | 5.0 (nametime stays 5.0) |
| min_units_ps | f32 | 100.0 (unused) |
| usercol | vec3 | 7.2 |
| accel | vec2 | zero-initialised via GLM_FORCE_CTOR_INIT (core/vectors.h:34); use (0,0) |

Constants (user.cpp:20-22): `gGourceBeamDist=100.0`, `gGourceActionDist=50.0`, `gGourcePersonalSpaceDist=100.0`. Settings defaults: `user_idle_time=3.0`, `user_friction=1.0`, `max_file_lag=5.0`, `max_user_speed=500`, `user_scale=1.0` (gource_settings.cpp:468-473).

### 7.1 Forces and movement
`applyForceUser(u)` (user.cpp:88-115): `dir=u.pos-pos; dist=length(dir)`; `desired = getActionCount()==0 ? 100 : (!actions.empty() && activeActions.empty()) ? 10 : 50` (`getActionCount = actionCount + activeCount`); if `dist<0.001`: `accel += 1.0f*normalise(vec2(rand()%100-50, rand()%100-50))`, return; if `dist<desired`: `accel -= (desired-dist)*normalise(dir)`.
`applyForceAction(a)` (user.cpp:117-141): `dir = target.abspos - pos; dist=length`; `dist<0.001` -> random nudge as above (no 1.0f factor); `dist<50` -> `accel -= (50-dist)*normalise(dir)`, return; `dist>100` -> `accel += (dist-100)*normalise(dir)`; between 50 and 100: none.
`applyForceToActions` (user.cpp:143-172): return if both lists empty; `last_action = elapsed` (prevents fade); apply `applyForceAction` for the first up to 3 active actions; if any active exist return; else apply for the **first** queued action only.
`Gource::interactUsers` (gource.cpp:1292-1333): user tree rebuilt each frame: bounds `user_bounds`+/-1, `max_depth` 1 or 6 (same `dir_bounds.area()>10000` test), max_items 1, items inserted in username order; for each user `a` in name order: `visitItemsInBounds(a.quadItemBounds, UserForceFunctor)` (skip self; dedupe when `node_count!=1`) -> `applyForceUser`; then `a->applyForceToActions()`. Forces read pre-move positions (positions only change in `logic`), so order only matters for float summation and `rand()` order. User bounds from `Pawn::updateQuadItemBounds` (size 20, ratio 1.333: half = 10 x 13.33).

`logic(t,dt)` (user.cpp:231-302):
1. `Pawn::logic(dt)`; `action_interval -= dt`; `find_nearby = !actions.empty() && action_interval<=0`.
2. Walk `actions` in order: if `max_file_lag>=0 && action.t < t - max_file_lag` (lagging): erase from queue, `rate=2.0`, push to active, `actionCount--, activeCount++`, continue scanning. Else if `!find_nearby` break. Else `d=|target.abspos - pos|`; if `d < 100` move to active (counts adjusted) and break; else next.
3. If `action_interval<=0`: `n=actionCount+activeCount; action_interval = n ? 1.0/(float)n : 1.0`.
4. For each active action in order: `action->logic(dt)`; if finished erase, delete, `activeCount--`.
5. `if length2(accel) > speed*speed: accel = normalise(accel)*speed`; `pos += accel*dt`; `accel = accel * max(0, 1 - user_friction*dt)` (accel persists across frames, decaying; new forces add to it).

### 7.2 Colour, alpha, lifetime
- `colourize()`: `usercol = colourHash(name)` (user.cpp:174-176). `assignUserImage` (user.cpp:178-221) calls it, sets usercol=(1,1,1) when a user/default image is used and `!colour_user_images`, then `usercol = usercol*0.6f + vec3(1)*0.4f; usercol *= 0.9f`. The recolour key calls `colourize()` alone, so afterwards user colours are raw hashes without the 0.6/0.4 mix or 0.9 factor (quirk).
- `getColour()`: selected -> white else usercol. `getAlpha()` (user.cpp:343-351): `a=min(elapsed/1,1)`; `if(elapsed-last_action > user_idle_time) a = 1 - min(elapsed-last_action-user_idle_time, 1.0)`.
- States: `isIdle = activeActions.empty() && actions.empty()`; `isFading = isIdle && elapsed-last_action > user_idle_time`; `isInactive = isIdle && elapsed-last_action > 10.0`. `updateUsers` (gource.cpp:1365-1416) runs `logic` for every user in name order, collects inactive users, then deletes them after the loop. Gource-global `idle_time` increases by dt when every user is idle (also when there are no users) else resets to 0.
- `last_action` refreshes only in `applyForceToActions`; a new user starts with `last_action=0` so does not fade until `elapsed > 3`.
- Highlight only affects name colour/visibility; selection tints white.

## 8. Hard-coded constants index

| Value | Meaning | Source |
|---|---|---|
| 3.14159265 | PI | core/pi.h:31 |
| 8.0 | `gGourceFileDiameter` | file.cpp:20 |
| 1.05 / 0.5 | file draw size factor / radius factor | file.cpp:29-30 |
| 5.0 | file approach speed | file.cpp:34 |
| 1.0 | file fade-in/out duration | pawn.cpp:38; file.cpp:169,206 |
| 15.0 (unused) | `gGourceMinDirSize` | dirnode.cpp:20 |
| 10.0 | `gGourceForceGravity` | dirnode.cpp:22 |
| 1.5 | `gGourceDirPadding` | dirnode.cpp:23 |
| 0.00001 | dir overlap epsilon | dirnode.cpp:624 |
| 0.6, 3.0 | dir brightness floor, brightness decay seconds | dirnode.cpp:548 |
| 5.0 | dir-name label fade seconds | dirnode.cpp:935-937 |
| 2.0 | spline clamp rate `dt*2` | dirnode.cpp:777 |
| 10000.0, 6, 1 | quadtree deep-mode area threshold, max depth, max items/node | gource.cpp:1308,1433,23,1312 |
| 100 / 50 / 100 | beam / action / personal-space distances | user.cpp:20-22 |
| 0.001 | user overlap epsilon | user.cpp:104,127 |
| 0.1, 0.5 | busy-user personal-space factors (pending-only 0.1; active 0.5) | user.cpp:99-101 |
| 3 | max active actions that attract a user | user.cpp:149 |
| 0.2 | initial `action_interval` | user.cpp:42 |
| 5.0 | initial user name_interval | user.cpp:43 |
| 10.0 | seconds idle before user deleted | user.cpp:362 |
| 20.0 | user base size | user.cpp:29 |
| 0.6, 0.4, 0.9 | user colour lighten/dim | user.cpp:219-220 |
| 0.5 / 2.0 / 10.0 | action base rate / lag rate / rate cap | action.cpp:21,35; user.cpp:250 |
| (0,1,0), (1,0,0), (1,0.7,0.3) | create / remove / modify beam colours | action.cpp:102,106,120 |
| 500, 5.0, 3.0, 1.0, 1.0 | defaults: max_user_speed, max_file_lag, user_idle_time, user_friction, user_scale | gource_settings.cpp:468-473 |
| 0.0, 4.0, 0.0, 0.75, 1.0 | defaults: elasticity, filename_time, file_idle_time, bloom_intensity, bloom_multiplier | gource_settings.cpp:460,487,411,437,436 |
| 0.1, 3.0 | defaults: days_per_second, auto_skip_seconds | gource_settings.cpp:410,409 |
| 1/60 | max interactive tick (exporter path uses 1/gource_fps instead; see timing.md) | gource.cpp:156 |
| 31 | default `gStringHashSeed`; recolour sets `rand()%10000+1` | stringhash.cpp:30; gource.cpp:1555 |

## 9. Randomness and time

| Use | Where | Notes |
|---|---|---|
| `rand()%100 - 50` pair, normalised | dirnode.cpp:625 (dir overlap <1e-5); user.cpp:106 (user-user <1e-3, factor 1.0f); user.cpp:128 (user-file <1e-3) | Common in practice: users spawned in one frame share a spawn point; dirs share (0,0) before first move. The order of `rand()` calls = force-application order. libc `rand()` is not portable; a Rust port needs a documented PRNG (or a glibc TYPE_3 clone) |
| `rand()%10000+1` | gource.cpp:1555 | recolour key only |
| `srand(time(0))`, `rand()%1000/1000.0f` | gource_settings.cpp:1319-1320 | random start position only |
| Wall clock | `SDL_GetTicks` only for debug timing (gource.cpp:263,1301,1426); frame dt from the SDL app loop | sim time is `runtime` (sum of clamped dt), `currtime` (commit timestamps + dt*86400*days_per_second) |
| No other randomness | - | other "pseudo-random" behaviour is string hashes (`vec2Hash`, `colourHash`) |

## 10. Float/int and ordering hazards for exact reproduction

1. Mixed f32/f64: C++ literals such as `0.5`, `PI`, `1.0/(float)n`, `diameter*PI`, `parent.radius*PI/(visible+1)`, `dt*86400.0*days_per_second` evaluate in f64 and are narrowed when stored in f32. Mirror per expression (see 3.4-3.5). `sinf/cosf/sqrt` operate on f32 values; `sqrt(float)` and `(float)sqrt(double)` agree.
2. `vec2` is glm 32-bit float; `glm::length` = `sqrtf(dot)`.
3. `stringHash` is XOR-based, signed-char, wrapping int. Use `i32` with `wrapping_*`, `(b as i8) as i32`, `31 ^ (n - i)`, and C truncating `/` and `%` (negative results only for the `i32::MIN` edge).
4. All maps iterate in sorted-key order; lists are insertion-ordered (never swap-remove).
5. `isVisible()` mutates `visible`; `getFilesRecursive` prepends; `findDirs` stops at the first prefix match.
6. Stale values: a child's `dir_area` as read by the parent; `quadItemBounds` of non-visible dirs; both depend on update order (`nodeUpdated` chain; `updateBounds` at frame start).
7. Uninitialised in C++: `RUser::accel`, `RFile::dest`, `RFile::touch_colour`, `RDirNode::in_frustum`, `node_count`. Treat as zero/false.
8. Fade/expire use `> 0.0f` tests (section 4 quirk).
9. Untouched files keep `elapsed=0`, `hidden=true`, and are not counted in `visible_count`/`dir_area`.
10. Time: interactive dt is clamped to 1/60 s and scaled by time_scale; the exporter path uses a fixed 1/gource_fps step (gource_fps = smallest multiple of output_framerate >= 60), NOT 1/60. timing.md is authoritative.

## 11. Rendering-relevant rules (visual equivalence only)

- Dir edges: spline from parent to child via `spos`, drawn for visible children; skipped for the root's direct children when `hide_root` (dirnode.cpp:1111,1124,1138). Spline class: src/spline.cpp (not catalogued here).
- Dir name label at `spline.getLabelPos()`; hidden if `hide_dirnames`, root, or `dir_name_depth>0 && dir_name_depth < depth-1`; alpha 1.0 if `highlight_dirs` else `max(0,5-since_last_node_change)/5`; hidden when `>5 s` (dirnode.cpp:930-943).
- Files: quads of size 8.4 with `file.png`, tint `getColour()`, alpha `getAlpha()`, shadow offset (2,2) at alpha*0.5; file name at abs pos + (5.5, -1) (selected -2); user name centred under the sprite (user.cpp:369-394).
- Name fade envelope: section 2.

## 12. Gaps / not read

- `src/spline.cpp/.h`, `src/bloom.*`, frustum, fxfont, camera (gource.cpp:1475-1550 only skimmed for rotation), `gource.h` member defaults, `file_key` (FileKey legend) - outside this subtask.
- Quadtree internals beyond what is cited (`getItemsAt`, `QuadNode::empty`) not read in detail (core/quadtree.cpp:135-300).
- `gource_settings.cpp` validators for `user_scale`, `max_files` etc. only grepped.

## Tick 3 corrections (from verification-tick3-scene.md)

- `time_inc` is `float` (gource.cpp:1723); app-core.md's f64 claim corrected.
- `toa` in ZoomCamera is `1.99999988f`, not 2.0, because the f64 literal 0.017453292 is narrowed (app-core.md:88).
- `file_area` as stored in the f32 field is 50.26548386 (section 3.1 quotes the f64 product 50.2654824).
- `accel`, `dest`, `touch_colour` are zero-initialised by GLM_FORCE_CTOR_INIT (core/vectors.h:34), not uninitialised.
- Only `since_node_visible` is guarded by `if(visible)` (dirnode.cpp:924-927); `since_last_file_change` / `since_last_node_change` always advance (section 3.5 wording).
- `RDirNode::split` and `calcArea` do not exist as functions: the regroup happens at the tail of `addFile`, and `dir_area` is a field.
- User-user quadtree query box is +/-(10, 13.33); with max_depth==1 every user interacts with every other, otherwise only users in overlapping leaves. The port must reproduce the quadtree, not approximate it.
- `vec2(rand()%100-50, rand()%100-50)` has unspecified argument evaluation order in C++; clang evaluates left-to-right in practice. The port must pin an order and the parity spec must record which libc/compiler produced the reference.
- Commits are never sorted anywhere; they are consumed in log order through a one-commit pushback buffer (see verification-tick3-scene.md).
