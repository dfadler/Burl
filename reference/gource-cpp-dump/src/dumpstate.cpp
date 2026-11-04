// State-dump fork additions (Burl). Not part of upstream Gource.

#include "dumpstate.h"

#include <cmath>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <fstream>
#include <sstream>

#include "core/conffile.h"
#include "gource.h"

DumpOptions gDump;

unsigned long long gDumpRandCalls = 0;
int gDumpRandLast = 0;
long long gDumpFirstRandFrame = -1;
int gDumpFrame = 0;

// ---- PRNG ----------------------------------------------------------------

// macOS libc rand(): Park-Miller minimal standard, seed 1 by default, returns the new state
// (srand(1) gives 16807, 282475249, 1622650073).
static int parkNext() {
    if(gDump.park_state == 0) gDump.park_state = gDump.have_seed ? (unsigned long long)(gDump.seed % 2147483647LL) : 1;
    if(gDump.park_state == 0) gDump.park_state = 1;
    gDump.park_state = (gDump.park_state * 16807ULL) % 2147483647ULL;
    return (int) gDump.park_state;
}

int gource_rand() {
    int r;

    if(gDump.script_set) {
        r = (int) gDump.script[gDump.script_pos++ % gDump.script.size()];
    } else if(gDump.prng == "park") {
        r = parkNext();
    } else {
        if(!gDump.libc_seeded) {
            gDump.libc_seeded = true;
            // glibc mode is srand(1) explicitly; default stays native unless a seed is given
            if(gDump.have_seed) srand((unsigned int) gDump.seed);
            else if(gDump.prng == "glibc") srand(1);
        }
        r = rand();
    }

    if(gDumpFirstRandFrame < 0) gDumpFirstRandFrame = gDumpFrame;
    gDumpRandCalls++;
    return gDumpRandLast = r;
}

void gource_srand_time() {
    if(gDump.prng == "default" && !gDump.script_set && !gDump.have_seed) srand(time(0));
}

static long long parseInt(const std::string& name, const std::string& value) {
    char* end = 0;
    long long v = strtoll(value.c_str(), &end, 10);
    if(value.empty() || *end != 0) throw ConfFileException(std::string("invalid ") + name + " value", "", 0);
    return v;
}

void dumpSetOption(const std::string& name, const std::string& value) {

    if(name == "debug-dump-state") {
        if(gDump.out) fclose(gDump.out);
        gDump.out = fopen(value.c_str(), "wb");
        if(!gDump.out) throw ConfFileException("could not write to '" + value + "'", "", 0);
        gDump.header_written = false;

    } else if(name == "debug-dump-every") {
        long long v = parseInt(name, value);
        if(v < 1) throw ConfFileException("invalid debug-dump-every value", "", 0);
        gDump.every = (int) v;

    } else if(name == "debug-dump-max-frames") {
        long long v = parseInt(name, value);
        if(v < 0) throw ConfFileException("invalid debug-dump-max-frames value", "", 0);
        gDump.max_frames = (int) v;

    } else if(name == "prng") {
        if(value != "default" && value != "glibc" && value != "park") throw ConfFileException("invalid prng value (default, glibc, park)", "", 0);
        gDump.prng = value;
        gDump.prng_set = true;
        gDump.park_state = 0;
        gDump.libc_seeded = false;
        if(gDump.script_set) throw ConfFileException("--prng and --rand-script cannot be combined", "", 0);

    } else if(name == "prng-seed") {
        gDump.seed = parseInt(name, value);
        gDump.have_seed = true;
        gDump.park_state = 0;
        gDump.libc_seeded = false;

    } else if(name == "rand-script") {
        std::ifstream in(value.c_str());
        if(!in) throw ConfFileException("could not read '" + value + "'", "", 0);

        std::vector<long long> values;
        std::string line;
        while(std::getline(in, line)) {
            if(line.find_first_not_of(" \t\r") == std::string::npos) continue;
            char* end = 0;
            long long v = strtoll(line.c_str(), &end, 10);
            while(*end == ' ' || *end == '\t' || *end == '\r') end++;
            if(*end != 0 || v < 0) throw ConfFileException("invalid value in rand-script '" + value + "'", "", 0);
            values.push_back(v);
        }
        if(values.empty()) throw ConfFileException("rand-script '" + value + "' is empty", "", 0);

        gDump.script = values;
        gDump.script_pos = 0;
        gDump.script_set = true;
        if(gDump.prng_set) throw ConfFileException("--prng and --rand-script cannot be combined", "", 0);

    } else if(name == "debug-pointer-script") {
        std::ifstream in(value.c_str());
        if(!in) throw ConfFileException("could not read '" + value + "'", "", 0);

        gDump.pointer.clear();
        std::string line;
        int last = 0;
        while(std::getline(in, line)) {
            size_t hash = line.find('#');
            if(hash != std::string::npos) line.erase(hash);
            std::istringstream ls(line);
            int frame;
            std::string kind;
            if(!(ls >> frame)) {
                if(line.find_first_not_of(" \t\r") == std::string::npos) continue;
                throw ConfFileException("invalid line in pointer script '" + value + "'", "", 0);
            }
            PointerEvent ev;
            ev.frame = frame; ev.kind = 0; ev.a = ev.b = 0;
            std::string arg;
            if(!(ls >> kind) || frame < last) throw ConfFileException("invalid line in pointer script '" + value + "'", "", 0);
            last = frame;
            if(kind == "move") {
                ev.kind = 'm';
                if(!(ls >> ev.a >> ev.b)) throw ConfFileException("invalid move in pointer script '" + value + "'", "", 0);
            } else if(kind == "down" || kind == "up") {
                ev.kind = kind == "down" ? 'd' : 'u';
                ls >> arg;
                if(arg == "l") ev.a = SDL_BUTTON_LEFT;
                else if(arg == "m") ev.a = SDL_BUTTON_MIDDLE;
                else if(arg == "r") ev.a = SDL_BUTTON_RIGHT;
                else throw ConfFileException("invalid button in pointer script '" + value + "'", "", 0);
            } else if(kind == "key") {
                ev.kind = 'k';
                ls >> arg;
                if(arg != "tab") throw ConfFileException("invalid key in pointer script '" + value + "'", "", 0);
            } else {
                throw ConfFileException("invalid event in pointer script '" + value + "'", "", 0);
            }
            gDump.pointer.push_back(ev);
        }

        gDump.pointer_pos = 0;
        gDump.pointer_active = true;
    }
}

// ---- JSON helpers ----------------------------------------------------------

void jstr(FILE* f, const std::string& s) {
    fputc('"', f);
    for(size_t i = 0; i < s.size(); i++) {
        unsigned char c = (unsigned char) s[i];
        if(c == '"' || c == '\\') { fputc('\\', f); fputc(c, f); }
        else if(c < 0x20) fprintf(f, "\\u%04x", c);
        else fputc(c, f);   // UTF-8 bytes pass through unchanged
    }
    fputc('"', f);
}

void jflt(FILE* f, float v) {
    if(std::isfinite(v)) fprintf(f, "%.9g", v);
    else fputs(std::isnan(v) ? "\"nan\"" : (v > 0 ? "\"inf\"" : "\"-inf\""), f);
}

void jvec2(FILE* f, const vec2& v) { fputc('[', f); jflt(f, v.x); fputc(',', f); jflt(f, v.y); fputc(']', f); }
void jvec3(FILE* f, const vec3& v) { fputc('[', f); jflt(f, v.x); fputc(',', f); jflt(f, v.y); fputc(',', f); jflt(f, v.z); fputc(']', f); }
void jvec4(FILE* f, const vec4& v) { fputc('[', f); jflt(f, v.x); fputc(',', f); jflt(f, v.y); fputc(',', f); jflt(f, v.z); fputc(',', f); jflt(f, v.w); fputc(']', f); }

void jbounds(FILE* f, const Bounds2D& b) {
    fputc('[', f);
    jflt(f, b.min.x); fputc(',', f); jflt(f, b.min.y); fputc(',', f);
    jflt(f, b.max.x); fputc(',', f); jflt(f, b.max.y); fputc(',', f);
    fputs(b.first ? "true" : "false", f);
    fputc(']', f);
}

static const char* jbool(bool b) { return b ? "true" : "false"; }

// ---- records -----------------------------------------------------------------

static void writeHeader() {
    if(gDump.header_written) return;
    gDump.header_written = true;

    FILE* f = gDump.out;
    const char* mode = gDump.script_set ? "script" : gDump.prng.c_str();

    fprintf(f, "{\"schema\":%d,\"type\":\"header\",\"prng\":\"%s\",\"prng_seed\":", DUMP_SCHEMA, mode);
    if(gDump.have_seed) fprintf(f, "%lld", gDump.seed); else fputs("null", f);
    fprintf(f, ",\"every\":%d,\"max_frames\":%d,\"compiler\":", gDump.every, gDump.max_frames);
#ifdef __VERSION__
    jstr(f, __VERSION__);
#else
    jstr(f, "unknown");
#endif
    fprintf(f, ",\"char_signed\":%s", jbool((char)-1 < 0));
#ifdef __FP_FAST_FMAF
    fputs(",\"fma_target\":true", f);
#else
    fputs(",\"fma_target\":false", f);
#endif
#ifdef __FLT_EVAL_METHOD__
    fprintf(f, ",\"flt_eval_method\":%d", (int) __FLT_EVAL_METHOD__);
#endif
    fputs("}\n", f);
}

// Written by GourceShell::getNext once a section's Gource exists: carries the per-section reset values.
void dumpSectionBoundary() {
    if(!gDump.out) return;
    writeHeader();

    gDump.section++;
    gDumpFrame = 0;

    FILE* f = gDump.out;
    fprintf(f, "{\"schema\":%d,\"type\":\"section\",\"section\":%d,\"repo_count\":%d,\"hash_seed\":%d,",
            DUMP_SCHEMA, gDump.section, gGourceSettings.repo_count, gStringHashSeed);
    fputs("\"stop_at_time\":", f);  jflt(f, gGourceSettings.stop_at_time);
    fputs(",\"stop_position\":", f); jflt(f, gGourceSettings.stop_position);
    fputs(",\"start_position\":", f); jflt(f, gGourceSettings.start_position);
    fputs(",\"path\":", f); jstr(f, gGourceSettings.path);
    fputs("}\n", f);
    fflush(f);
}

void RDirNode::dumpJson(FILE* f) const {
    fputs("{\"path\":", f); jstr(f, abspath);
    fputs(",\"parent\":", f); if(parent) jstr(f, parent->abspath); else fputs("null", f);
    fputs(",\"pos\":", f); jvec2(f, pos);
    fputs(",\"spos\":", f); jvec2(f, spos);
    fputs(",\"accel\":", f); jvec2(f, accel);
    fputs(",\"prev_accel\":", f); jvec2(f, prev_accel);
    fputs(",\"dir_area\":", f); jflt(f, dir_area);
    fputs(",\"file_area\":", f); jflt(f, file_area);
    fputs(",\"dir_radius\":", f); jflt(f, dir_radius);
    fputs(",\"parent_radius\":", f); jflt(f, parent_radius);
    fprintf(f, ",\"visible\":%s,\"position_initialized\":%s,\"visible_count\":%d,\"depth\":%d",
            jbool(visible), jbool(position_initialized), visible_count, depth);
    fputs(",\"since_node_visible\":", f); jflt(f, since_node_visible);
    fputs(",\"since_last_file_change\":", f); jflt(f, since_last_file_change);
    fputs(",\"since_last_node_change\":", f); jflt(f, since_last_node_change);
    fputs(",\"col\":", f); jvec4(f, col);
    fputs(",\"quad_bounds\":", f); jbounds(f, quadItemBounds);
    fputc('}', f);
}

void RFile::dumpJson(FILE* f) const {
    fputs("{\"path\":", f); jstr(f, fullpath);
    fputs(",\"dir\":", f); if(dir) jstr(f, dir->getPath()); else fputs("null", f);
    fputs(",\"pos\":", f); jvec2(f, pos);
    fputs(",\"dest\":", f); jvec2(f, dest);
    fputs(",\"distance\":", f); jflt(f, distance);
    fputs(",\"accel\":", f); jvec2(f, accel);
    fputs(",\"elapsed\":", f); jflt(f, elapsed);
    fputs(",\"alpha\":", f); jflt(f, getAlpha());
    fprintf(f, ",\"hidden\":%s,\"removing\":%s,\"expired\":%s,\"forced_removal\":%s",
            jbool(hidden), jbool(removing), jbool(expired), jbool(forced_removal));
    fputs(",\"fade_start\":", f); jflt(f, fade_start);
    fputs(",\"last_action\":", f); jflt(f, last_action);
    fprintf(f, ",\"removed_timestamp\":%lld", (long long) removed_timestamp);
    fputs(",\"file_colour\":", f); jvec3(f, file_colour);
    fputs(",\"touch_colour\":", f); jvec3(f, touch_colour);
    fputs(",\"colour\":", f); jvec3(f, getColour());
    fputs(",\"size\":", f); jflt(f, size);
    fputs(",\"name_interval\":", f); jflt(f, name_interval);
    fprintf(f, ",\"tagid\":%d}", tagid);
}

static void dumpAction(FILE* f, const RAction* a, bool active) {
    const char* kind = dynamic_cast<const CreateAction*>(a) ? "A" : dynamic_cast<const RemoveAction*>(a) ? "D" : "M";
    fputs("{\"file\":", f); jstr(f, a->target->fullpath);
    fprintf(f, ",\"kind\":\"%s\",\"timestamp\":%lld", kind, (long long) a->timestamp);
    fputs(",\"t\":", f); jflt(f, a->t);
    fputs(",\"progress\":", f); jflt(f, a->progress);
    fputs(",\"rate\":", f); jflt(f, a->rate);
    fprintf(f, ",\"active\":%s}", jbool(active));
}

void RUser::dumpJson(FILE* f) const {
    bool idle = activeActions.empty() && actions.empty();

    fputs("{\"name\":", f); jstr(f, name);
    fputs(",\"pos\":", f); jvec2(f, pos);
    fputs(",\"accel\":", f); jvec2(f, accel);
    fputs(",\"speed\":", f); jflt(f, speed);
    fputs(",\"size\":", f); jflt(f, size);
    fputs(",\"elapsed\":", f); jflt(f, elapsed);
    fputs(",\"alpha\":", f); jflt(f, getAlpha());
    fputs(",\"last_action\":", f); jflt(f, last_action);
    fputs(",\"action_interval\":", f); jflt(f, action_interval);
    fputs(",\"name_interval\":", f); jflt(f, name_interval);
    fputs(",\"usercol\":", f); jvec3(f, usercol);
    fprintf(f, ",\"highlighted\":%s,\"queued\":%u,\"active\":%u", jbool(highlighted), (unsigned) actions.size(), (unsigned) activeActions.size());
    // same expressions as isIdle/isFading/isInactive, read-only
    fprintf(f, ",\"idle\":%s,\"fading\":%s,\"inactive\":%s,\"tagid\":%d",
            jbool(idle),
            jbool(idle && (elapsed - last_action) > gGourceSettings.user_idle_time),
            jbool(idle && (elapsed - last_action) > 10.0),
            tagid);
    fputs(",\"actions\":[", f);
    bool first = true;
    for(std::list<RAction*>::const_iterator it = actions.begin(); it != actions.end(); it++) {
        if(!first) fputc(',', f);
        first = false;
        dumpAction(f, *it, false);
    }
    for(std::list<RAction*>::const_iterator it = activeActions.begin(); it != activeActions.end(); it++) {
        if(!first) fputc(',', f);
        first = false;
        dumpAction(f, *it, true);
    }
    fputs("]}", f);
}

void ZoomCamera::dumpJson(FILE* f) const {
    fputs("{\"pos\":", f); jvec3(f, pos);
    fputs(",\"dest\":", f); jvec3(f, dest);
    fputs(",\"target\":", f); jvec3(f, target);
    fprintf(f, ",\"lockon\":%s,\"lockon_time\":", jbool(lockon)); jflt(f, lockon_time);
    fputc('}', f);
}

void RCaption::dumpJson(FILE* f) const {
    fputs("{\"text\":", f); jstr(f, caption);
    fprintf(f, ",\"timestamp\":%lld,\"elapsed\":", (long long) timestamp); jflt(f, elapsed);
    fputs(",\"alpha\":", f); jflt(f, alpha);
    fputc('}', f);
}

static void dumpCaptions(FILE* f, const std::list<RCaption*>& l) {
    fputc('[', f);
    for(std::list<RCaption*>::const_iterator it = l.begin(); it != l.end(); it++) {
        if(it != l.begin()) fputc(',', f);
        (*it)->dumpJson(f);
    }
    fputc(']', f);
}

// Called from Gource::update after logic() and before draw(). Read-only: no isVisible(), no rand().
void Gource::dumpState() {
    FILE* f = gDump.out;

    if(!commitlog) return;   // still loading

    if(dump_loaded_frame0 < 0) {
        dump_loaded_frame0 = framecount;
        dump_runtime_at_load = runtime;
    }

    bool last = appFinished || (gDump.max_frames > 0 && framecount + 1 >= gDump.max_frames);

    if(framecount == dump_loaded_frame0 || framecount % gDump.every == 0 || last) {

        writeHeader();

        fprintf(f, "{\"schema\":%d,\"type\":\"frame\",\"section\":%d,\"repo_count\":%d,\"frame\":%d,\"g\":{",
                DUMP_SCHEMA, gDump.section, gGourceSettings.repo_count, framecount);
        fprintf(f, "\"currtime\":%lld,\"lasttime\":%lld", (long long) currtime, (long long) lasttime);
        fputs(",\"subseconds\":", f); jflt(f, subseconds);
        fputs(",\"runtime\":", f); jflt(f, runtime);
        fputs(",\"idle_time\":", f); jflt(f, idle_time);
        fprintf(f, ",\"paused\":%s,\"stop_position_reached\":%s,\"commitqueue_len\":%u",
                jbool(paused), jbool(stop_position_reached), (unsigned) commitqueue.size());
        fputs(",\"last_percent\":", f); jflt(f, last_percent);
        fputs(",\"rotation_remaining_angle\":", f); jflt(f, rotation_remaining_angle);
        fputs(",\"rotate_angle\":", f); jflt(f, rotate_angle);
        fprintf(f, ",\"hash_seed\":%d,\"rand_calls\":%llu,\"rand_last\":%u,\"first_rand_frame\":%lld",
                gStringHashSeed, gDumpRandCalls, (unsigned) gDumpRandLast, gDumpFirstRandFrame);
        fprintf(f, ",\"loaded_frame0\":%d,\"runtime_at_load\":", dump_loaded_frame0); jflt(f, dump_runtime_at_load);
        fprintf(f, ",\"n_dirs\":%u,\"n_files\":%u,\"n_users\":%u,\"n_removed_pending\":%u,\"tag_seq\":%d",
                (unsigned) gGourceDirMap.size(), (unsigned) files.size(), (unsigned) users.size(),
                (unsigned) gGourceRemovedFiles.size(), tag_seq);
        fputs(",\"dir_bounds\":", f); jbounds(f, dir_bounds);
        fputs(",\"user_bounds\":", f); jbounds(f, user_bounds);
        if(gDump.pointer_active) dumpPointerJson(f);

        fputs("},\"camera\":", f); camera.dumpJson(f);

        // std::map iteration is already sorted by key (abspath, fullpath, name)
        fputs(",\"dirs\":[", f);
        for(std::map<std::string,RDirNode*>::const_iterator it = gGourceDirMap.begin(); it != gGourceDirMap.end(); it++) {
            if(it != gGourceDirMap.begin()) fputc(',', f);
            it->second->dumpJson(f);
        }
        fputs("],\"files\":[", f);
        for(std::map<std::string,RFile*>::const_iterator it = files.begin(); it != files.end(); it++) {
            if(it != files.begin()) fputc(',', f);
            it->second->dumpJson(f);
        }
        fputs("],\"users\":[", f);
        for(std::map<std::string,RUser*>::const_iterator it = users.begin(); it != users.end(); it++) {
            if(it != users.begin()) fputc(',', f);
            it->second->dumpJson(f);
        }
        fputs("],\"captions\":{\"queued\":", f);
        dumpCaptions(f, captions);
        fputs(",\"active\":", f);
        dumpCaptions(f, active_captions);
        fputs("}}\n", f);
    }

    if(last) {
        fflush(f);
        appFinished = true;   // cap reached (or already finished)
    }
}

// ---- scripted pointer (--debug-pointer-script) -------------------------------

// The scripted button mask stands in for SDL_GetMouseState, which injected events never change.
bool Gource::buttonLeft() const {
    return gDump.pointer_active ? (gDump.pointer_buttons & SDL_BUTTON(SDL_BUTTON_LEFT)) != 0 : cursor.leftButtonPressed();
}

bool Gource::buttonRight() const {
    return gDump.pointer_active ? (gDump.pointer_buttons & SDL_BUTTON(SDL_BUTTON_RIGHT)) != 0 : cursor.rightButtonPressed();
}

bool Gource::buttonAny() const {
    return gDump.pointer_active ? (gDump.pointer_buttons & (SDL_BUTTON(SDL_BUTTON_LEFT) | SDL_BUTTON(SDL_BUTTON_RIGHT))) != 0
                                : cursor.buttonPressed();
}

// MouseCursor::isVisible without the window-focus test (Xvfb has no pointer): not hidden and
// moved or clicked within the last 3 s (MouseCursor::timeout), counted in fixed ticks.
bool Gource::dumpCursorVisible() const {
    if(!gDump.pointer_active) return cursor.isVisible();
    return !cursor.isHidden() && gDump.pointer_idle < 3.0f;
}

void Gource::dumpPointer() {
    while(gDump.pointer_pos < gDump.pointer.size() && gDump.pointer[gDump.pointer_pos].frame <= framecount) {
        const PointerEvent& ev = gDump.pointer[gDump.pointer_pos++];

        if(ev.kind == 'm') {
            SDL_MouseMotionEvent e;
            memset(&e, 0, sizeof(e));
            e.type = SDL_MOUSEMOTION;
            e.x = ev.a; e.y = ev.b;
            e.xrel = ev.a - gDump.pointer_x; e.yrel = ev.b - gDump.pointer_y;
            gDump.pointer_x = ev.a; gDump.pointer_y = ev.b;
            mouseMove(&e);

        } else if(ev.kind == 'd' || ev.kind == 'u') {
            int bit = SDL_BUTTON(ev.a);
            if(ev.kind == 'd') gDump.pointer_buttons |= bit; else gDump.pointer_buttons &= ~bit;
            SDL_MouseButtonEvent e;
            memset(&e, 0, sizeof(e));
            e.type = ev.kind == 'd' ? SDL_MOUSEBUTTONDOWN : SDL_MOUSEBUTTONUP;
            e.button = ev.a;
            mouseClick(&e);

        } else {
            SDL_KeyboardEvent e;
            memset(&e, 0, sizeof(e));
            e.type = SDL_KEYDOWN;
            e.keysym.sym = SDLK_TAB;
            keyPress(&e);
        }
    }
}

static void jnamed(FILE* f, const char* key, const std::string* name) {
    fprintf(f, ",\"%s\":", key);
    if(name) jstr(f, *name); else fputs("null", f);
}

void Gource::dumpPointerJson(FILE* f) const {
    fputs(",\"pointer\":{\"mousepos\":", f); jvec2(f, mousepos);
    fprintf(f, ",\"buttons\":%d,\"grab\":%s,\"dragged\":%s", gDump.pointer_buttons, jbool(grab_mouse), jbool(mousedragged));
    jnamed(f, "hover_user", hoverUser ? &hoverUser->getName() : 0);
    jnamed(f, "hover_file", hoverFile ? &hoverFile->fullpath : 0);
    jnamed(f, "selected_user", selectedUser ? &selectedUser->getName() : 0);
    jnamed(f, "selected_file", selectedFile ? &selectedFile->fullpath : 0);
    fputc('}', f);
}
