// Describes the machine's main display for `display-harness --real-panel`; creates no window and
// never takes focus. Prints one line:
//   panel scale=<backingScaleFactor> bounds=<w>x<h> pixels=<w>x<h> name=<localized name>
// (bounds in points, pixels physical). Exits 3 with a message when there is no display at all
// (headless, SSH without a window server session).
#import <Cocoa/Cocoa.h>
int main(void) {
  @autoreleasepool {
    [NSApplication sharedApplication];  // connects to the window server; shows nothing
    NSScreen *s = NSScreen.screens.firstObject;  // screens[0] is the main display (menu bar)
    if (!s) {
      fprintf(stderr, "no display: no window server session or no attached screen\n");
      return 3;
    }
    CGDirectDisplayID id = [s.deviceDescription[@"NSScreenNumber"] unsignedIntValue];
    CGRect b = CGDisplayBounds(id);
    printf("panel scale=%g bounds=%.0fx%.0f pixels=%zux%zu name=%s\n", s.backingScaleFactor,
           b.size.width, b.size.height, CGDisplayPixelsWide(id), CGDisplayPixelsHigh(id),
           s.localizedName.UTF8String);
  }
  return 0;
}
