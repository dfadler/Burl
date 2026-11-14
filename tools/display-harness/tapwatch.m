// Listen-only mouse event observer for the cursor lab (`cargo xtask cursor-lab`).
//   tapwatch SECONDS [hid|session|annotated]
// Creates a listen-only CGEventTap (default: kCGSessionEventTap) for mouse moved / dragged / down /
// up events and prints one line per event for SECONDS:
//   event type=moved x=412.0 y=390.0 dx=10 dy=0
// First line `access trusted=.. listen=..` (AXIsProcessTrusted, CGPreflightListenEventAccess);
// `ready` once the tap runs; `tap-create-failed` and exit 3 if CGEventTapCreate returned NULL
// (Input Monitoring / Accessibility missing). Never modifies or blocks an event.
#import <Cocoa/Cocoa.h>

static CGEventRef cb(CGEventTapProxy proxy, CGEventType type, CGEventRef e, void *ctx) {
  const char *name = "other";
  switch (type) {
    case kCGEventMouseMoved: name = "moved"; break;
    case kCGEventLeftMouseDragged: name = "dragged"; break;
    case kCGEventLeftMouseDown: name = "down"; break;
    case kCGEventLeftMouseUp: name = "up"; break;
    default: break;
  }
  CGPoint p = CGEventGetLocation(e);
  printf("event type=%s x=%.1f y=%.1f dx=%lld dy=%lld\n", name, p.x, p.y,
         CGEventGetIntegerValueField(e, kCGMouseEventDeltaX),
         CGEventGetIntegerValueField(e, kCGMouseEventDeltaY));
  fflush(stdout);
  return e;
}

int main(int argc, char **argv) {
  if (argc < 2) return 2;
  double secs = atof(argv[1]);
  CGEventTapLocation loc = kCGSessionEventTap;
  if (argc > 2) {
    if (!strcmp(argv[2], "hid")) loc = kCGHIDEventTap;
    else if (!strcmp(argv[2], "annotated")) loc = kCGAnnotatedSessionEventTap;
    else if (strcmp(argv[2], "session")) return 2;
  }
  printf("access trusted=%d listen=%d\n", (int)AXIsProcessTrusted(), (int)CGPreflightListenEventAccess());
  fflush(stdout);
  CGEventMask mask = CGEventMaskBit(kCGEventMouseMoved) | CGEventMaskBit(kCGEventLeftMouseDragged) |
                     CGEventMaskBit(kCGEventLeftMouseDown) | CGEventMaskBit(kCGEventLeftMouseUp);
  CFMachPortRef tap = CGEventTapCreate(loc, kCGHeadInsertEventTap, kCGEventTapOptionListenOnly, mask, cb, NULL);
  if (!tap) {
    printf("tap-create-failed\n");
    return 3;
  }
  CFRunLoopSourceRef src = CFMachPortCreateRunLoopSource(NULL, tap, 0);
  CFRunLoopAddSource(CFRunLoopGetCurrent(), src, kCFRunLoopCommonModes);
  CGEventTapEnable(tap, true);
  printf("ready\n");
  fflush(stdout);
  CFRunLoopRunInMode(kCFRunLoopDefaultMode, secs, false);
  return 0;
}
