// State-dump fork additions (Burl). Not part of upstream Gource.
// Hidden CommandLine options: --debug-dump-state FILE, --debug-dump-every N,
// --debug-dump-max-frames N, --prng default|glibc|park, --rand-script FILE, --prng-seed N.
// See ../README.md for the JSONL schema.

#ifndef DUMPSTATE_H
#define DUMPSTATE_H

#include <cstdio>
#include <string>
#include <vector>

#include "core/bounds.h"
#include "core/vectors.h"

#define DUMP_SCHEMA 1

// One line of the pointer script: `FRAME move X Y`, `FRAME down|up l|m|r`, `FRAME key tab`.
struct PointerEvent {
    int frame;
    char kind;                       // 'm' move, 'd' down, 'u' up, 'k' key
    int a, b;                        // move: x, y; down/up: button (1 left, 2 middle, 3 right)
};

struct DumpOptions {
    FILE* out;                       // 0 unless --debug-dump-state given
    int every;
    int max_frames;                  // 0 = unlimited
    std::string prng;                // "default", "glibc", "park" or "script"
    bool prng_set, script_set;
    bool have_seed;
    long long seed;
    std::vector<long long> script;   // --rand-script values, cyclic
    size_t script_pos;
    unsigned long long park_state;   // Park-Miller state, 0 = not seeded yet
    bool libc_seeded;
    int section;                     // 0 until the first section boundary
    bool header_written;
    // --debug-pointer-script: scripted mouse/key events (see ../README.md, "Scripted pointer")
    bool pointer_active;
    std::vector<PointerEvent> pointer;
    size_t pointer_pos;
    int pointer_buttons;             // SDL_BUTTON(n): bit 0 left, 1 middle, 2 right
    float pointer_idle;              // seconds since the last scripted move/click (cursor timeout)
    int pointer_x, pointer_y;
    DumpOptions() : out(0), every(1), max_frames(0), prng("default"), prng_set(false),
                    script_set(false), have_seed(false), seed(0), script_pos(0),
                    park_state(0), libc_seeded(false), section(0), header_written(false),
                    pointer_active(false), pointer_pos(0), pointer_buttons(0), pointer_idle(3.0f),
                    pointer_x(0), pointer_y(0) {}
};

extern DumpOptions gDump;

// All rand() sites in the simulation route through here.
extern unsigned long long gDumpRandCalls;
extern int gDumpRandLast;
extern long long gDumpFirstRandFrame;   // Gource::framecount of the first call, -1 until then
extern int gDumpFrame;                  // current Gource::framecount, set by Gource::update

int gource_rand();
void gource_srand_time();               // replaces srand(time(0)); no-op unless prng is default and unseeded

// option handling; throw ConfFileException on bad input
void dumpSetOption(const std::string& name, const std::string& value);

// JSON helpers (fixed key order is the caller's job)
void jstr(FILE* f, const std::string& s);
void jflt(FILE* f, float v);
void jvec2(FILE* f, const vec2& v);
void jvec3(FILE* f, const vec3& v);
void jvec4(FILE* f, const vec4& v);
void jbounds(FILE* f, const Bounds2D& b);

void dumpSectionBoundary();

#endif
