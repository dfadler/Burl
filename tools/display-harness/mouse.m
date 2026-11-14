// Synthetic mouse input for the cursor-grab check (`cargo xtask display-harness --grab`) and the
// cursor lab (`cargo xtask cursor-lab`).
//   mouse pos                      print the cursor position, `X Y` (points, global top-left)
//   mouse warp X Y                 put the cursor at X Y and send a move there
//   mouse move X Y DX DY           a mouse-moved event at X Y carrying delta DX DY
//   mouse down X Y | up X Y        left button press / release at X Y
//   mouse drag X Y DX DY           a left-drag event at X Y carrying delta DX DY
//   mouse rel DX DY                RELATIVE-only: an event whose location is the CURRENT cursor
//                                  position, carrying only the delta fields (type: --type)
//   mouse vis                      print `visible=1|0` from CGCursorIsVisible (deprecated API)
//   mouse bounds                   print `W H` of the main display, in points
// Options, anywhere on the line (default: tap hid, no source, type moved):
//   --tap hid|session|annotated   where CGEventPost injects: kCGHIDEventTap, kCGSessionEventTap,
//                                  kCGAnnotatedSessionEventTap
//   --source hid                   build the event from CGEventSourceCreate(HIDSystemState)
//   --type moved|dragged           the event type `rel` posts
// Posted events carry an absolute location, and on hosted macos-15 and macos-26 the cursor follows
// it even while burl holds a Locked grab (CGAssociateMouseAndMouseCursorPosition(false)), so `pos`
// cannot tell a lock from none. The check asserts burl's `grab:` log lines instead.
// Needs Accessibility, as CGEventPost does.
#import <Cocoa/Cocoa.h>

static CGEventTapLocation g_tap = kCGHIDEventTap;
static CGEventSourceRef g_source = NULL;
static CGEventType g_rel_type = kCGEventMouseMoved;

static void post_event(CGEventRef e) {
  CGEventPost(g_tap, e);
  CFRelease(e);
}

static void post(CGEventType type, double x, double y, int dx, int dy) {
  CGEventRef e = CGEventCreateMouseEvent(g_source, type, CGPointMake(x, y), kCGMouseButtonLeft);
  CGEventSetIntegerValueField(e, kCGMouseEventDeltaX, dx);
  CGEventSetIntegerValueField(e, kCGMouseEventDeltaY, dy);
  post_event(e);
}

static CGPoint current(void) {
  CGEventRef e = CGEventCreate(NULL);
  CGPoint p = CGEventGetLocation(e);
  CFRelease(e);
  return p;
}

int main(int argc, char **argv) {
  // Pull the options out; what is left is the command and its numbers.
  char *args[16];
  int n = 0;
  for (int i = 1; i < argc && n < 16; i++) {
    if (!strcmp(argv[i], "--tap") && i + 1 < argc) {
      const char *v = argv[++i];
      if (!strcmp(v, "hid")) g_tap = kCGHIDEventTap;
      else if (!strcmp(v, "session")) g_tap = kCGSessionEventTap;
      else if (!strcmp(v, "annotated")) g_tap = kCGAnnotatedSessionEventTap;
      else return 2;
    } else if (!strcmp(argv[i], "--source") && i + 1 < argc) {
      if (strcmp(argv[++i], "hid")) return 2;
      g_source = CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
    } else if (!strcmp(argv[i], "--type") && i + 1 < argc) {
      const char *v = argv[++i];
      if (!strcmp(v, "moved")) g_rel_type = kCGEventMouseMoved;
      else if (!strcmp(v, "dragged")) g_rel_type = kCGEventLeftMouseDragged;
      else return 2;
    } else {
      args[n++] = argv[i];
    }
  }
  if (n < 1) return 2;
  const char *cmd = args[0];
  double x = n > 2 ? atof(args[1]) : 0, y = n > 2 ? atof(args[2]) : 0;
  int dx = n > 4 ? atoi(args[3]) : 0, dy = n > 4 ? atoi(args[4]) : 0;
  if (!strcmp(cmd, "pos")) {
    CGPoint p = current();
    printf("%.0f %.0f\n", p.x, p.y);
  } else if (!strcmp(cmd, "warp")) {
    CGWarpMouseCursorPosition(CGPointMake(x, y));
    post(kCGEventMouseMoved, x, y, 0, 0);
  } else if (!strcmp(cmd, "move")) {
    post(kCGEventMouseMoved, x, y, dx, dy);
  } else if (!strcmp(cmd, "down")) {
    post(kCGEventLeftMouseDown, x, y, 0, 0);
  } else if (!strcmp(cmd, "up")) {
    post(kCGEventLeftMouseUp, x, y, 0, 0);
  } else if (!strcmp(cmd, "drag")) {
    post(kCGEventLeftMouseDragged, x, y, dx, dy);
  } else if (!strcmp(cmd, "rel")) {
    // `rel DX DY`: args[1], args[2] are the deltas, not a location.
    CGPoint p = current();
    post(g_rel_type, p.x, p.y, n > 2 ? atoi(args[1]) : 0, n > 2 ? atoi(args[2]) : 0);
  } else if (!strcmp(cmd, "vis")) {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    printf("visible=%d\n", (int)CGCursorIsVisible());
#pragma clang diagnostic pop
  } else if (!strcmp(cmd, "bounds")) {
    CGRect b = CGDisplayBounds(CGMainDisplayID());
    printf("%.0f %.0f\n", b.size.width, b.size.height);
  } else {
    return 2;
  }
  return 0;
}
