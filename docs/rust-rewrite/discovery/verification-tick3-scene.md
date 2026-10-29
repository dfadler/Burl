# Verification tick 3: scene / simulation logic, independently re-derived

Method: the pseudocode in section 2 was written from `src/*.cpp` only, before reading `scene-model.md` and `app-core.md`; the table in section 1 compares the two afterwards. Paths are relative to the repo root. `scene-model.md` and `app-core.md` were not edited.

## 1. Discrepancies (doc says X, source says Y)

| # | Doc | Doc says | Source says | Cite | Impact |
|---|---|---|---|---|---|
| 1 | scene-model.md 2 (`accel`), 3.1 (`in_frustum`), 7, 10.7 | `RUser::accel`, `RFile::dest`, `RFile::touch_colour` are "uninitialised in C++, treat as zero" | They are **defined** zero: `GLM_FORCE_CTOR_INIT` is on, so a default `vec2/vec3` member is zero. Not UB. (`QuadItem::node_count` is a plain `int`, truly uninit until `QuadTree::addItem` sets 0.) | src/core/vectors.h:34; src/pawn.h:37; src/file.h:29,44; src/core/quadtree.cpp:497 | Wording; port's zero start is correct and guaranteed. |
| 2 | app-core.md 242, 356 | `time_inc` is "f64 then `(int)` truncation" | `float time_inc = (dt * 86400.0 * days_per_second)`: computed in f64, **stored f32** before `(int)` and before `time_inc - (float)seconds`. scene-model.md 0 step 3 is right; the two docs disagree. | src/gource.cpp:1723-1726 | Commit timing at the f32 rounding edge. Use f32. |
| 3 | app-core.md 614 | head-of-line `break` at gource.cpp:1757 | `if(commit.timestamp > currtime) break;` is at **1754** (1756 is `processCommit`). | src/gource.cpp:1754 | Cite only. |
| 4 | app-core.md 88 | `toa = tan(fov*0.5*DEGREES_TO_RADIANS)*2.0` "(=2.0 at 90deg)" | `DEGREES_TO_RADIANS` is the f64 literal `0.017453292` (not pi/180): `tan(0.78539814)*2 = 1.9999999064`, stored in `float toa` = **1.99999988** (numerically verified). | src/zoomcamera.cpp:121; src/core/pi.h:33 | `dest.z` ~6e-8 relative larger; matters only for bit-exact camera. |
| 5 | scene-model.md 3.1 | `file_area` = "16*3.14159265 = 50.2654824" | That is the f64 product; the field is `float`, stored **50.26548386**. | src/dirnode.cpp:55-57 | Seeds `dir_area`/`dir_radius` for every dir. |
| 6 | scene-model.md 3.5 `logic` | `if(visible) since_node_visible+=dt; since_last_file_change+=dt; since_last_node_change+=dt` (reads as all guarded) | Only `since_node_visible` is under `if(visible)` (no braces); the other two advance **unconditionally**. | src/dirnode.cpp:924-927 | Render only; misleading text. |
| 7 | brief / scene-model.md 3.2 | `RDirNode::split`, `calcArea` | Neither function exists. "Split" = the common-prefix regroup at the tail of `addFile` (doc 3.2 step 9 describes it correctly); area = the `dir_area` field set in `calcRadius`. | src/dirnode.cpp:456-493, 574-594 | Terminology. |
| 8 | scene-model.md 7.1 | user-user forces come "via quadtree" | Confirmed, with a sharper consequence: the query box is only `pos +/- (10, 13.33)`, and every user in any overlapping **leaf** is visited (leaf capacity 1, depth 1 or 6). With `max_depth==1` (dir_bounds area <= 10000) the root is a single leaf, so **all** users interact; otherwise only users in leaves overlapping that small box. The port must reproduce the quadtree, not a radius query. | src/gource.cpp:1305-1326; src/core/quadtree.cpp:38,248-264 | Behavioural. |
| 9 | scene-model.md 9 | `rand()` order = force-application order | True, plus: in `vec2(rand()%100-50, rand()%100-50)` the **evaluation order of the two calls is unspecified** in C++ (gcc usually right-to-left, clang left-to-right); the reference libc/compiler must be pinned. | src/user.cpp:106,128; src/dirnode.cpp:625 | Matters whenever two users share a point (every new user spawns at `dir_bounds.centre()`) or dirs coincide. |

Checked and **consistent** with the docs: RUser::logic and the three applyForce functions, lag promotion (rate 2.0), action_interval reset, friction; RFile::logic (incl. `fade_start>0` quirk, `elapsed=0` while hidden), touch/remove; RAction::logic and subclasses; RDirNode applyForces/applyForceDir/move/updateSplinePoint/setInitialPosition/calcRadius (incl. `parent_radius` floor applied after the x1.5); addFile/removeFile/addNode/regroup; updateFilePositions/calcFileDest; DirForceFunctor skip set; quadtree depth rule; ZoomCamera adjust/logic/setPos; per-frame order and container orders. **Commits are never sorted**: consumed strictly in log order (only the one-commit pushback buffer and `--start-date` skip-forward exist; sorting appears only in UI/key code).

## 2. Independently derived pseudocode (second source)


Conventions: `f32` = C++ `float`, `f64` = `double`. glm `vec2/vec3` are f32 vectors; `GLM_FORCE_CTOR_INIT` is on (src/core/vectors.h:34) so a default `vec2` is (0,0). `PI` is the macro `3.14159265` (an **f64 literal with 8 decimals, not f64 pi**; src/core/pi.h:31). `DEGREES_TO_RADIANS` is `0.017453292` (f64 literal; pi.h:33). `f32 op f64-literal` is computed in f64 and rounded to f32 on assignment; those spots are marked (f64).
`normalise(v)`: `l = length(v) (f32); if l > 0.0 return v / l; else return v` (src/core/vectors.cpp:7-11). A zero vector stays zero. `length2` = x*x+y*y.
Containers are `std::list` (iteration = insertion order) unless noted.

### Pawn (src/pawn.cpp)
```
Pawn ctor(name,pos,tagid): hidden=false; speed=1.0; selected=false; shadow=false; elapsed=0.0;
   nametime=5.0; name_interval=0.0; graphic_ratio=1.0; size=0.0          // pawn.cpp:22-46
Pawn::logic(dt):                                                           // pawn.cpp:74-80
   elapsed += dt                       // f32
   if !hidden && name_interval > 0.0: name_interval -= dt
Pawn::showName(): if name_interval <= 0.0: name_interval = nametime        // :60
```

### RUser (src/user.cpp). Globals f32: gGourceBeamDist=100, gGourceActionDist=50, gGourcePersonalSpaceDist=100 (:20-22)
```
ctor: speed = settings.max_user_speed (default 500); size = 20.0*user_scale (f64->f32);
      last_action=0.0; action_interval=0.2; name_interval=5.0; actionCount=activeCount=0     // :24-48
addAction(a): if a.source != this return; if isIdle() showName();
      actions.push_back(a); actionCount++                                  // :50-59
isIdle() = activeActions.empty() && actions.empty()                        // :353
getActionCount() = actionCount + activeCount                               // :223
getPendingActionCount() = actionCount                                      // :227

applyForceUser(u):                                                          // :88-115
   if u == this return
   dir = u.pos - pos; dist = length(dir)                    // f32
   desired = getActionCount()==0 ? 100.0
           : (!actions.empty() && activeActions.empty()) ? 100.0*0.1 (f64->f32 = 10.0)
           : 100.0*0.5 (=50.0)
   if dist < 0.001: accel += 1.0f * normalise(vec2(rand()%100 - 50, rand()%100 - 50)); return
   if dist < desired: accel -= (desired - dist) * normalise(dir)
   // The two rand() calls are function-argument expressions: evaluation order is unspecified in C++.

applyForceAction(action):                                                   // :117-141
   f_pos = action.target.getAbsolutePos()  // = file.pos + file.dir.pos
   dir = f_pos - pos; dist = length(dir); desired = 50.0
   if dist < 0.001: accel += normalise(vec2(rand()%100-50, rand()%100-50)); return
   if dist < 50:  accel -= (50 - dist) * normalise(dir); return
   if dist > 100: accel += (dist - 100) * normalise(dir)
   // between 50 and 100: no force

applyForceToActions():                                                      // :143-172
   if activeActions.empty() && actions.empty() return    // last_action NOT refreshed
   last_action = elapsed
   n = 0
   for a in activeActions: applyForceAction(a); n++; if n >= 3 break
   if !activeActions.empty() return
   applyForceAction(actions.front())      // only the first pending action

RUser::logic(t, dt):                                                        // :231-302
   Pawn::logic(dt)                                   // elapsed += dt
   action_interval -= dt
   find_nearby = !actions.empty() && action_interval <= 0
   for it in actions (erase-safe):
        a = *it
        if max_file_lag >= 0.0 && a.t < t - max_file_lag:      // default max_file_lag = 5.0
             erase; actionCount--; a.rate = 2.0; activeActions.push_back(a); activeCount++; continue
        if !find_nearby: break
        d = length(a.target.getAbsolutePos() - pos)
        if d < 100.0 (gGourceBeamDist): erase; activeActions.push_back(a); actionCount--; activeCount++; break
        it++                                          // else keep scanning
   if action_interval <= 0:
        total = actionCount + activeCount
        action_interval = total ? (1.0 / (f32)total)(f64->f32) : 1.0
   for it in activeActions (erase-safe):
        a.logic(dt); if a.isFinished(): erase; delete a; activeCount--
   if length2(accel) > speed*speed: accel = normalise(accel) * speed
   pos += accel * dt
   accel = accel * max(0.0f, 1.0f - user_friction*dt)   // user_friction setting stored as 1.0/value (gource_settings.cpp:1421), default 1.0
```
Order: all users get `applyForceUser` (quadtree neighbours) then `applyForceToActions` in `Gource::interactUsers` (gource.cpp:1321-1330), THEN `updateUsers` calls `RUser::logic` per user in `std::map<string,RUser*>` (name-sorted) order (gource.cpp:1371-1374). `accel` therefore persists (decayed by friction) between frames.

### RFile (src/file.cpp, file.h). gGourceFileDiameter = 8.0f (file.cpp:20)
```
ctor: hidden = true; size = 8.0*1.05 (f64->f32); radius = size*0.5; speed = 5.0;
      nametime = settings.filename_time; name_interval = nametime; last_action=0; fade_start=-1.0;
      removed_timestamp=0; expired=forced_removal=removing=false; distance=0; dest=(0,0)  // :27-71
setDest(v): dest = v; setDistance(d): distance = d                          // file.h:76-77  (there is NO updatePos/ setDest logic beyond these setters)
getAbsolutePos() = pos + dir.pos                                           // :96

RFile::logic(dt):                                                           // :175-225
   Pawn::logic(dt)                                   // elapsed += dt ; name_interval-- only if !hidden
   dest_pos = dest * distance                        // target ring position
   accel = dest_pos - pos
   accel2 = accel * speed * dt                       // (accel*5.0)*dt, left-to-right
   if length2(accel2) > length2(accel): accel2 = accel
   pos += accel2
   accel = (0,0)
   if fade_start < 0.0 && file_idle_time > 0.0 && (elapsed - last_action) > file_idle_time: fade_start = elapsed
   if fade_start > 0.0 && !expired && (elapsed - fade_start) >= 1.0:
        expired = true; push this onto gGourceRemovedFiles unless already present
   if isHidden() && !forced_removal: elapsed = 0.0
   // quirk: fade_start == 0.0 exactly is neither <0 nor >0 -> that file never expires

touch(ts, colour):                                                          // :227-252
   if forced_removal || (removing && ts < removed_timestamp) return
   fade_start = -1; removing=false; removed_timestamp=0; last_action = elapsed; touch_colour = colour
   if expired: erase this from gGourceRemovedFiles; expired=false
   showName(); setHidden(false); dir.fileUpdated(true)
setHidden(h): if this.hidden==true && h==false && dir!=0: dir.addVisible()    // :254  (visible_count++, visible=true)
remove(ts):  last_action = elapsed; fade_start = elapsed; removing = true; removed_timestamp = ts   // :76
remove():    forced_removal = true; remove(0)                               // :83
```
Expired files are deleted at the start of the next `Gource::logic` (after readLog and currtime advance, before commit processing: gource.cpp:1736-1740).

### RAction (src/action.cpp, action.h)
```
ctor: progress = 0.0; rate = 0.5                                           // :20-22
logic(dt):                                                                  // :28-38
   if progress >= 1.0 return
   if progress == 0.0: apply()                       // target.touch(timestamp, colour)
   action_rate = min(10.0, rate * max(1.0, (f32)source.getPendingActionCount()))
   progress = min(progress + action_rate*dt, 1.0)
CreateAction: colour (0,1,0), no override.  RemoveAction: colour (1,0,0)
RemoveAction::logic(dt): old = progress; RAction::logic(dt); if old < 1.0 && progress >= 1.0: target.remove(timestamp)   // :109-117
ModifyAction: colour (1,0.7,0.3); apply(): RAction::apply(); target.setFileColour(modify_colour)   // :119-126
isFinished() = progress >= 1.0   (action.h:44)
```
Lag-promoted actions get rate=2.0 (user.cpp:250). Rate multiplier uses the user's *pending* count (not active), min(10, ...).

### ZoomCamera (src/zoomcamera.cpp)
ctor(pos,target,min,max): fov=90.0, znear=0.1, up=(0,-1,0), padding=1.0, speed=1.0, lockon=false, lockon_time=0, dest=pos, zfar=max+1.0   (:23-38)
```
adjust(bounds, adjust_distance=true):                                       // :96-154
   centre = bounds.min + (bounds.max-bounds.min)*0.5f
   dest.x = centre.x; dest.y = centre.y
   if !adjust_distance return                      // dest.z untouched
   width = bounds.width()*padding; height = bounds.height()*padding
   aspect = display.width / (f32)display.height
   if aspect < 1.0: height /= aspect else width /= aspect
   toa = tan(fov*0.5f*DEGREES_TO_RADIANS)*2.0     // f64 const 0.017453292; stored f32 = 1.99999988 (NOT 2.0; verified numerically)
   if crop_vertical: distance = width/toa
   elif crop_horizontal: distance = height/toa
   else distance = (width >= height) ? width/toa : height/toa
   if distance < min_distance: distance=min; if distance > max_distance: distance=max
   dest.z = -distance
logic(dt):                                                                  // :170-188
   dp  = dest - pos
   dpt = dp * dt * speed                           // (dp*dt)*speed
   if lockon:
        dpt = dpt*lockon_time + dp*(1.0f - lockon_time)
        if lockon_time > 0.0: lockon_time = max(0.0f, lockon_time - dt*0.5f)
   if length2(dpt) > length2(dp): dpt = dp
   pos += dpt
   target = (pos.x, pos.y, 0.0)
setPos(p, keep_angle): if keep_angle { dir = target - pos; pos = p; target = p + dir } else pos = p   // :160-168
setDistance(d): dest.z = -d;  stop(): dest = pos
```
Caller (gource.cpp:1475-1522): non-manual: `cambounds` = selected user/file bounds if track_users && selection, else `active_user_bounds` when `track_users && idle_time==0`, else `dir_bounds`; `camera.adjust(cambounds, !manual_zoom)`; then `camera.logic(dt)`. Manual: `cam_rate = -pos.z/5000.0`, `cursor_delta = cursor_move*cam_rate*10.0`, `setPos(cam_pos,true)`, `stop()`. (camera.speed set elsewhere via setSpeed; ctor default 1.0.)

### RDirNode (src/dirnode.cpp). Globals f32: gGourceForceGravity=10.0, gGourceDirPadding=1.5
```
ctor(parent_arg, abspath):                                                  // :34-70
   changePath(abspath)  // appends trailing '/', registers in gGourceDirMap (std::map<string,RDirNode*>, path-sorted)
   parent = 0; setParent(0)   // the parent ARG IS IGNORED (param reassigned to 0 on line 38): depth=1, path_token unset
   accel=spos=prev_accel=vel=(0,0); pos=(0,0)       // the `if(parent!=0)` init branch is dead code
   file_area = (4.0f*4.0f*PI)(f64->f32) = 50.26548386 as f32 (f64 product 50.2654824)   // padded_file_radius = gGourceFileDiameter*0.5 at ctor time
   visible_count=0; visible=false; position_initialized=false; since_*=0
   calcRadius(); calcColour()
setParent(p): if p!=0 && this.parent==p return; parent=p; adjustPath(); adjustDepth()     // :219
```
```
calcRadius():                                                               // :574-594
   total_file_area = file_area * visible_count            // f32*int
   dir_area = total_file_area
   for child in children: dir_area += child.dir_area      // child's CURRENT stored dir_area (may be stale)
   dir_radius   = max(1.0f, (f32)sqrt(dir_area)) * gGourceDirPadding          // x1.5f
   parent_radius = max(1.0f, (f32)sqrt(total_file_area) * gGourceDirPadding)  // NOTE: 1.0 floor applied AFTER the x1.5 here
(no calcArea function: getArea() returns dir_area; dir_radius_sqrt is never computed)

fileUpdated(ui): calcRadius(); since_last_file_change=0; nodeUpdated(ui)
nodeUpdated(ui): if ui: since_last_node_change = 0; calcRadius(); updateFilePositions();
                 if visible && noDirs() && noFiles(): visible=false;   if parent: parent.nodeUpdated(true)   // :107-114
addVisible(): visible_count++; visible=true
isVisible(): if visible true; if any child.isVisible(): visible=true (latched), true; else false
empty() = visible_count==0 && noDirs()
```
```
addFile(f) -> bool:                                                         // :377-496
   if !f.path.startsWith(abspath):                  // find(abspath)!=0
        if parent != 0 return false
        common = commonPathPrefix(f.path)   // longest common prefix of abspath/f.path, cut after the last '/' seen within the matched chars; "" if none
        if common == "": common = "/"
        newparent = new RDirNode(0, common); newparent.addNode(this); return newparent.addFile(f)
        // caller (gource.cpp:1013-1016) walks root up while root.getParent()!=0
   if parent==0 && abspath=="/" && f.path != abspath && noFiles() && noDirs(): changePath(f.path)   // empty root adopts path
   if f.path == abspath: files.push_back(f); if !f.hidden: visible_count++; f.setDir(this); fileUpdated(false); return true
   added=false
   for child in children: added = child.addFile(f); if added break
   if added && parent != 0 return true
   for file in files: if f.path.startsWith(file.fullpath): file.remove() [forced_removal=true]; break   // file is actually a dir
   if added return true                              // (only root reaches here when added)
   node = new RDirNode(this, f.path); node.addFile(f); addNode(node)
   // regroup: first child c (list order) with common=c.commonPathPrefix(f.path), common.size()>abspath.size() && common != f.path
   if found: cnode = new RDirNode(this, common); cnode.setPos(c.pos);
        for child in children (erase-safe): if child.prefixedBy(common): erase; cnode.addNode(child)
        addNode(cnode)
   return true
addNode(node):                                                              // :241-267
   for child in children (erase-safe): if child.prefixedBy(node.path): erase; node.addNode(child)
        // prefixedBy(p): abspath.find(p + '/' unless p ends '/') == 0
   children.push_back(node); node.setParent(this); nodeUpdated(false)
removeFile(f) -> bool:                                                      // :275-320
   if !f.path.startsWith(abspath) return false
   if f.path == abspath: find f in files; erase; if !f.hidden: visible_count--; fileUpdated(false); return true  (not found: false)
   for child: if child.removeFile(f): { if child.noFiles() && child.noDirs(): erase child; delete child; nodeUpdated(false) }; return true
   return false
```
There is NO `RDirNode::split`; the nearest equivalent is the common-prefix regroup at the tail of `addFile`. Only dirs with no files AND no children are reaped; `removeFile` never touches the `visible` latch.

```
updateFilePositions():                                                      // :842-880  (files in insertion order)
   max_files=1; diameter=1; file_no=0; d=0.0 (f32); files_left = visible_count
   for f in files:
        if f.hidden: f.setDest((0,0)); f.setDistance(0); continue
        f.setDest(calcFileDest(max_files, file_no)); f.setDistance(d)
        files_left--; file_no++
        if file_no >= max_files:
             diameter++; d += gGourceFileDiameter(8.0)
             max_files = (int) max(1.0, diameter*PI)            // f64 trunc: 6, 9, 12, 15, 18, 21 ...
             if files_left < max_files: max_files = files_left
             file_no = 0
calcFileDest(max_files, file_no):                                           // :831-840
   arc  = (f32)(1.0 / (f32)max_files)                  // f64 divide, f32 store
   frac = (f32)(arc*0.5 + arc*file_no)                 // f64 arithmetic
   return vec2(sinf(frac*PI*2.0), cosf(frac*PI*2.0))   // arg computed in f64, narrowed to f32 for sinf/cosf
// first file: max_files=1,file_no=0 -> frac=0.5 -> (sinf(pi)~=-8.7e-8, -1), distance 0 => sits at dir centre
```
```
applyForces(quadtree):   // :647-714 ; called once on root per frame (gource.cpp:1452)
   for child in children: child.applyForces(quadtree)        // DEPTH-FIRST, children BEFORE self
   if parent == 0 return                                      // root gets no forces
   visit quadtree items whose leaves overlap this.quadItemBounds (pos +/- radius, set in Gource::updateBounds: STALE, visible nodes only) with DirForceFunctor:
        skip: d==this, d==this.parent, d.parent==this, this.isParent(d) [d is ancestor], d.isParent(this) [d is descendant]
        if d.node_count != 1: skip if already seen this visit (std::set)
        this.applyForceDir(d)
   applyForceDir(parent)    // always
   parent_dist = distanceToParent() = length(parent.pos - pos) - (dir_radius + parent.parent_radius)
   accel += 10.0 * parent_dist * normalise(parent.pos - pos)            // (10*parent_dist) f32, then * vec
   pp = parent.parent
   if pp != 0:
        n = normalise(parent.pos - pp.pos)
        accel += (parent.pos + (parent.dir_radius + this.dir_radius) * n) - pos
   sib_accel = (0,0); visible = 1
   for node in parent.children (list order): if node==this continue; if !node.isVisible() continue
        visible++; sib_accel -= normalise(node.pos - pos)
   if visible > 1: slice = (parent.dir_radius * PI)(f64) / (f32)(visible+1) -> f32; sib_accel *= slice; accel += sib_accel
applyForceDir(node):                                                        // :604-630
   if node==this return
   dir = node.pos - pos; posd2 = length2(dir); sum = dir_radius_this + dir_radius_node
   distance2 = posd2 - sum*sum; if distance2 > 0.0 return          // only overlapping circles interact
   posd = sqrt(posd2); distance = posd - r_this - r_node           // negative
   if posd < 0.00001: accel += normalise(vec2(rand()%100-50, rand()%100-50)); return
   accel += distance * normalise(dir)                              // pushes away
move(dt):                                                                   // :799-825
   if parent==0 return
   if !empty() && !position_initialized: setInitialPosition()
   pos += accel*dt
   if elasticity > 0.0: diff = accel - prev_accel; m = dt*elasticity; a3 = prev_accel*(1.0f-m) + diff*m; pos += a3; prev_accel = a3
   accel = (0,0)           // default elasticity 0.0
setInitialPosition():                                                       // :780-797
   pos = parent.pos
   if parent.parent != 0: pos += normalise( normalise(parent.pos - parent.parent.pos)*2.0f + vec2Hash(abspath) )
   else:                  pos += vec2Hash(abspath)
   spos = pos - (parent.pos - pos)*0.5f; position_initialized = true
updateSplinePoint(dt):                                                      // :762-778
   if parent==0 return
   td = (parent.pos - pos)*0.5f; mid = pos + td; delta = mid - spos
   if length2(delta) > length2(td): spos += normalise(delta) * (length(delta) - length(td))
   spos += delta * min(1.0f, dt*2.0f)                       // delta NOT recomputed after the first adjustment
logic(dt):                                                                  // :895-928
   move(dt); updateSplinePoint(dt)
   if parent: node_normal = normalise(pos - parent.pos)
   for f in files: f.logic(dt)
   for child in children: child.logic(dt)      // PRE-ORDER: this node moves before its children
   calcColour()  (render only)
   if visible: since_node_visible += dt
   since_last_file_change += dt; since_last_node_change += dt       // (unconditional: the `if` has no braces)
```
Per-frame order inside `Gource::logic` (gource.cpp:1867-1875): `updateBounds` -> `interactUsers` -> `updateUsers(t,dt)` -> `interactDirs` (rebuilds quadtree from `gGourceDirMap` in path-sorted order, only `!empty()` nodes) -> `updateDirs`: `root.applyForces` (ALL forces first) then `root.logic` (move+files) -> `updateCamera(dt)` -> `updateTime`. `updateBounds` considers only `isVisible()` dirs. Quadtree depth = 1 if `dir_bounds.area() <= 10000` else 6 (gource.cpp:1305-1310, 1430-1435; node capacity 1; root counts as depth 1).

Quadtree (src/core/quadtree.cpp): leaf capacity 1; on a 2nd item a leaf splits (if depth < max) into 4 children in order TL,TR,BL,BR; an item goes to every child whose bounds `overlaps` (non-strict) so one item can be in several leaves (node_count>1 -> `seen` dedupe). `visitItemsInBounds` visits ALL items of every non-empty leaf that overlaps the query bounds (child order 0..3, list order within a leaf); the force functions then gate on distance themselves. f32 `accel +=` accumulation order therefore depends on this visit order (insertion order: users name-sorted via std::map; dirs path-sorted via gGourceDirMap). For user-user forces the query bounds are pos +/- (size/2, size/2*graphic_ratio) (= +/-(10, 13.33) for the stock 384x512 user.png) so only users sharing/overlapping leaves are ever considered (not "all within 100").

### Commit ordering
No sorting of commits anywhere: `std::sort` only appears in UI/key code (core/ui/ui.cpp, core/ui/file_selector.cpp, key.cpp:273). `RCommitLog::nextCommit` (formats/commitlog.cpp:216-236) yields commits in file order (one-commit pushback buffer `lastCommit/buffered`); `RCommit::postprocess` only runs `filter_utf8` on the username (:353); `isValid` applies user filters and requires non-empty files (:357-384). `RLogMill` (logmill.cpp:86-97) with `--start-date` scans in file order to the first `timestamp >= start_timestamp` and buffers it. `Gource::readLog` (gource.cpp:1131-1154) appends to `commitqueue` (deque) in log order while queue empty or (`back().timestamp <= currtime` and `size < commitqueue_max_size=100`), stops at the first commit with `timestamp > stop_timestamp`. `Gource::logic` pops from the FRONT while `front.timestamp <= currtime` (gource.cpp:1744-1774), so an out-of-order later commit is processed in log position, and (without `--no-time-travel`) sets `currtime = commit.timestamp` when `lasttime > commit.timestamp` (:1765-1767). Time advance: `float time_inc = dt*86400.0*days_per_second` (computed f64, STORED f32: gource.cpp:1723), `seconds=(int)time_inc`, `subseconds(f32) += time_inc - (f32)seconds`, carry when >= 1.0 (:1723-1733); first commit seeds `currtime=lasttime=queue[0].timestamp` (:1715). `rand()` is unseeded unless `--start-position random` (gource_settings.cpp:1319) -> platform libc default sequence.
