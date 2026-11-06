// Prints `<windowID> <x> <y> <w> <h>` (points, global top-left) for each on-screen window of the
// given owner name taller than 100 points (burl opens a small stray extra window).
#import <Cocoa/Cocoa.h>
int main(int argc, char **argv) {
  if (argc < 2) return 2;
  NSString *owner = [NSString stringWithUTF8String:argv[1]];
  NSArray *all = CFBridgingRelease(CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID));
  for (NSDictionary *w in all) {
    if (![w[@"kCGWindowOwnerName"] isEqual:owner]) continue;
    CGRect r;
    CGRectMakeWithDictionaryRepresentation((__bridge CFDictionaryRef)w[@"kCGWindowBounds"], &r);
    if (r.size.height > 100)
      printf("%d %.0f %.0f %.0f %.0f\n", [w[@"kCGWindowNumber"] intValue], r.origin.x, r.origin.y, r.size.width, r.size.height);
  }
  return 0;
}
