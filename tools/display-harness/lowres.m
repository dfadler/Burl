// Negative control for `cargo xtask crisp-control`: a 640x360 pt window with an NSOpenGLView that
// draws a fixed high-frequency test card, no shaders. Usage: lowres YES|NO
//   YES: wantsBestResolutionOpenGLSurface = YES, a native 2x drawable on a 2x display.
//   NO:  a 1x drawable (640x360 px) that the window server magnifies on a 2x display.
// The property is set before the view is first displayed. The card is drawn in drawable pixels, so
// the drawing commands are the same either way; only the drawable size differs. Prints
// `drawable WxH` (pixels) whenever it changes. Runs until killed or until its parent exits.
// NSOpenGLView and OpenGL are deprecated (macOS 10.14) but are still shipped; unverified on 26.
#import <Cocoa/Cocoa.h>
#import <OpenGL/gl.h>
#include <unistd.h>

@interface Card : NSOpenGLView
@end

@implementation Card {
  int lastW, lastH;
}

static void rect(int x, int y, int w, int h) {
  glVertex2i(x, y); glVertex2i(x + w, y); glVertex2i(x + w, y + h); glVertex2i(x, y + h);
}

- (void)drawRect:(NSRect)dirty {
  [[self openGLContext] makeCurrentContext];
  NSSize px = [self convertSizeToBacking:self.bounds.size];
  int W = (int)px.width, H = (int)px.height;
  if (W != lastW || H != lastH) {
    lastW = W; lastH = H;
    printf("drawable %dx%d\n", W, H);
  }
  glViewport(0, 0, W, H);
  glMatrixMode(GL_PROJECTION); glLoadIdentity(); glOrtho(0, W, 0, H, -1, 1);
  glMatrixMode(GL_MODELVIEW); glLoadIdentity();
  glDisable(GL_BLEND); glDisable(GL_MULTISAMPLE); glDisable(GL_DITHER);
  glClearColor(0.08f, 0.08f, 0.08f, 1); glClear(GL_COLOR_BUFFER_BIT);
  glColor3f(0.92f, 0.92f, 0.92f);
  glBegin(GL_QUADS);
  // Vertical lines: 1 px every 8 px, 2 px every 16 px (offset 4) across the left 2/3.
  for (int x = 0; x < W * 2 / 3; x += 8) rect(x, 0, 1, H / 2);
  for (int x = 4; x < W * 2 / 3; x += 16) rect(x, H / 2, 2, H / 2);
  // Horizontal lines on the right 1/3: 1 px every 8 px (top half), 2 px every 16 px (bottom half).
  for (int y = 0; y < H / 2; y += 8) rect(W * 2 / 3, y, W / 3, 1);
  for (int y = H / 2 + 4; y < H; y += 16) rect(W * 2 / 3, y, W / 3, 2);
  // Diagonals, 1 px wide, both slopes plus a steep one.
  for (int i = 0; i < H / 3; i++) { rect(20 + i, 20 + i, 1, 1); rect(20 + H / 3 - i, 20 + i + H / 2, 1, 1); }
  for (int i = 0; i < H / 3; i++) rect(120 + i / 2, H / 2 + 20 + i, 1, 1);
  // Fine checker, 1 px squares, 64x64.
  for (int y = 0; y < 64; y++)
    for (int x = 0; x < 64; x++)
      if ((x + y) & 1) rect(W - 100 + x, H - 100 + y, 1, 1);
  glEnd();
  glFlush();
  [[self openGLContext] flushBuffer];
}
@end

int main(int argc, char **argv) { @autoreleasepool {
  if (argc != 2 || (strcmp(argv[1], "YES") && strcmp(argv[1], "NO"))) {
    fprintf(stderr, "usage: lowres YES|NO\n");
    return 2;
  }
  BOOL best = !strcmp(argv[1], "YES");
  setvbuf(stdout, NULL, _IONBF, 0);
  pid_t parent = getppid();
  [NSApplication sharedApplication];
  [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
  NSOpenGLPixelFormatAttribute attrs[] = { NSOpenGLPFADoubleBuffer, NSOpenGLPFAColorSize, 24, 0 };
  NSOpenGLPixelFormat *pf = [[NSOpenGLPixelFormat alloc] initWithAttributes:attrs];
  NSRect frame = NSMakeRect(100, 100, 640, 360);
  Card *view = [[Card alloc] initWithFrame:frame pixelFormat:pf];
  view.wantsBestResolutionOpenGLSurface = best;  // before the view is first displayed
  NSWindow *win = [[NSWindow alloc] initWithContentRect:frame
                                              styleMask:NSWindowStyleMaskTitled
                                                backing:NSBackingStoreBuffered
                                                  defer:NO];
  win.title = best ? @"lowres native" : @"lowres 1x";
  win.releasedWhenClosed = NO;
  win.contentView = view;
  [NSApp finishLaunching];
  [win orderFront:nil];  // no activation: do not take focus from the user
  [NSTimer scheduledTimerWithTimeInterval:0.5 repeats:YES block:^(NSTimer *t) {
    if (getppid() != parent) exit(0);
    view.needsDisplay = YES;
  }];
  [NSApp run];
  return 0;
}}
