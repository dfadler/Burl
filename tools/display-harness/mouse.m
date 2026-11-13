// Synthetic mouse input for the cursor-grab check (`cargo xtask display-harness --grab`).
//   mouse pos                      print the cursor position, `X Y` (points, global top-left)
//   mouse warp X Y                 put the cursor at X Y and send a move there
//   mouse move X Y DX DY           a mouse-moved event at X Y carrying delta DX DY
//   mouse down X Y | up X Y        left button press / release at X Y
//   mouse drag X Y DX DY           a left-drag event at X Y carrying delta DX DY
// Posted events carry an absolute location, and on hosted macos-15 and macos-26 the cursor follows
// it even while burl holds a Locked grab (CGAssociateMouseAndMouseCursorPosition(false)), so `pos`
// cannot tell a lock from none. The check asserts burl's `grab:` log lines instead.
// Needs Accessibility, as CGEventPost does.
#import <Cocoa/Cocoa.h>

static void post(CGEventType type, double x, double y, int dx, int dy) {
  CGEventRef e = CGEventCreateMouseEvent(NULL, type, CGPointMake(x, y), kCGMouseButtonLeft);
  CGEventSetIntegerValueField(e, kCGMouseEventDeltaX, dx);
  CGEventSetIntegerValueField(e, kCGMouseEventDeltaY, dy);
  CGEventPost(kCGHIDEventTap, e);
  CFRelease(e);
}

int main(int argc, char **argv) {
  if (argc < 2) return 2;
  const char *cmd = argv[1];
  double x = argc > 3 ? atof(argv[2]) : 0, y = argc > 3 ? atof(argv[3]) : 0;
  int dx = argc > 5 ? atoi(argv[4]) : 0, dy = argc > 5 ? atoi(argv[5]) : 0;
  if (!strcmp(cmd, "pos")) {
    CGEventRef e = CGEventCreate(NULL);
    CGPoint p = CGEventGetLocation(e);
    CFRelease(e);
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
  } else {
    return 2;
  }
  return 0;
}
