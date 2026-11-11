// Display helper for `cargo xtask display-harness`: creates one 2x (1920x1080 points, 3840x2160
// pixels) and one 1x (1280x720) virtual display, prints a `ready` line once NSScreen lists both
// with the right backing scale, then runs until SIGTERM/SIGINT or until its parent exits.
// Uses the private CGVirtualDisplay API (declared by hand); see README.md for the caveats.
#import <Cocoa/Cocoa.h>
#include <signal.h>
#include <unistd.h>

@interface CGVirtualDisplayMode : NSObject
- (instancetype)initWithWidth:(NSUInteger)w height:(NSUInteger)h refreshRate:(double)r;
@end
@interface CGVirtualDisplaySettings : NSObject
@property(nonatomic) unsigned int hiDPI;
@property(nonatomic, copy) NSArray *modes;
@end
@interface CGVirtualDisplayDescriptor : NSObject
@property(nonatomic) unsigned int vendorID, productID, serialNum;
@property(nonatomic, copy) NSString *name;
@property(nonatomic) CGSize sizeInMillimeters;
@property(nonatomic) unsigned int maxPixelsWide, maxPixelsHigh;
@property(nonatomic) CGPoint redPrimary, greenPrimary, bluePrimary, whitePoint;
@property(nonatomic, strong) dispatch_queue_t queue;
@end
@interface CGVirtualDisplay : NSObject
@property(nonatomic, readonly) unsigned int displayID;
- (instancetype)initWithDescriptor:(CGVirtualDisplayDescriptor *)d;
- (BOOL)applySettings:(CGVirtualDisplaySettings *)s;
@end

static volatile sig_atomic_t stop = 0;
static void on_signal(int s) { (void)s; stop = 1; }

// Without pumped AppKit events NSScreen never lists the new displays.
static void pump(double sec) {
  NSDate *end = [NSDate dateWithTimeIntervalSinceNow:sec];
  while ([end timeIntervalSinceNow] > 0) {
    NSEvent *ev = [NSApp nextEventMatchingMask:NSEventMaskAny
                                     untilDate:[NSDate dateWithTimeIntervalSinceNow:0.05]
                                        inMode:NSDefaultRunLoopMode dequeue:YES];
    if (ev) [NSApp sendEvent:ev];
  }
}

static NSScreen *screen_for(CGDirectDisplayID id) {
  for (NSScreen *s in NSScreen.screens)
    if ([s.deviceDescription[@"NSScreenNumber"] unsignedIntValue] == id) return s;
  return nil;
}

// Pump until `ok` holds; false on timeout or signal.
static BOOL wait_for(double timeout, BOOL (^ok)(void)) {
  NSDate *end = [NSDate dateWithTimeIntervalSinceNow:timeout];
  while (!ok()) {
    if (stop || [end timeIntervalSinceNow] <= 0) return NO;
    pump(0.1);
  }
  return YES;
}

static CGVirtualDisplay *make(NSString *name, unsigned serial, unsigned w, unsigned h, double mmw, double mmh, BOOL hidpi,
                              NSArray *modes) {
  CGVirtualDisplayDescriptor *d = [CGVirtualDisplayDescriptor new];
  // macOS 14+ needs a unique non-zero vendor/serial pair per display.
  d.name = name; d.vendorID = 0x4275; d.productID = 0x1000 + serial; d.serialNum = serial;
  d.maxPixelsWide = w; d.maxPixelsHigh = h;
  d.sizeInMillimeters = CGSizeMake(mmw, mmh);
  d.queue = dispatch_get_main_queue();
  CGVirtualDisplay *vd = [[CGVirtualDisplay alloc] initWithDescriptor:d];
  CGVirtualDisplaySettings *s = [CGVirtualDisplaySettings new];
  s.hiDPI = hidpi; s.modes = modes;
  if (!vd || ![vd applySettings:s]) return nil;
  return vd;
}

// Switch `id` to the mode that is `width` points and `pixelWidth` pixels wide.
static BOOL switch_mode(CGDirectDisplayID did, size_t width, size_t pixelWidth) {
  __block CGDisplayModeRef want = NULL;
  NSDictionary *opt = @{ (id)kCGDisplayShowDuplicateLowResolutionModes : @YES };
  if (!wait_for(20, ^BOOL {
        CFArrayRef ms = CGDisplayCopyAllDisplayModes(did, (__bridge CFDictionaryRef)opt);
        if (!ms) return NO;
        for (CFIndex i = 0; i < CFArrayGetCount(ms); i++) {
          CGDisplayModeRef m = (CGDisplayModeRef)CFArrayGetValueAtIndex(ms, i);
          if (CGDisplayModeGetWidth(m) == width && CGDisplayModeGetPixelWidth(m) == pixelWidth) {
            want = CGDisplayModeRetain(m);
            break;
          }
        }
        CFRelease(ms);
        return want != NULL;
      }))
    return NO;
  CGDisplayConfigRef cfg;
  CGBeginDisplayConfiguration(&cfg);
  CGConfigureDisplayWithDisplayMode(cfg, did, want, NULL);
  return CGCompleteDisplayConfiguration(cfg, kCGConfigureForAppOnly) == kCGErrorSuccess;
}

static int fail(const char *msg) { fprintf(stderr, "vd: %s\n", msg); return 1; }

int main(void) { @autoreleasepool {
  setvbuf(stdout, NULL, _IONBF, 0);
  signal(SIGTERM, on_signal); signal(SIGINT, on_signal);
  pid_t parent = getppid();
  [NSApplication sharedApplication];
  [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
  [NSApp finishLaunching];

  CGVirtualDisplayMode *(^mode)(NSUInteger, NSUInteger) = ^(NSUInteger w, NSUInteger h) {
    return [[CGVirtualDisplayMode alloc] initWithWidth:w height:h refreshRate:60];
  };
  CGVirtualDisplay *hi = make(@"burl-2x", 1, 3840, 2160, 600, 340, YES, @[ mode(3840, 2160), mode(1920, 1080) ]);
  CGVirtualDisplay *lo = make(@"burl-1x", 2, 1280, 720, 300, 170, NO, @[ mode(1280, 720) ]);
  if (!hi || !lo) return fail("could not create the virtual displays");
  CGDirectDisplayID hid = hi.displayID, lid = lo.displayID;

  // hiDPI only advertises the 2x mode: switch into 1920x1080 points / 3840x2160 pixels. The 1x
  // display defaults to 1024x576, so switch it to 1280x720 too.
  if (!switch_mode(hid, 1920, 3840)) return fail("could not switch the hiDPI display to 1920x1080 @2x");
  if (!switch_mode(lid, 1280, 1280)) return fail("could not switch the 1x display to 1280x720");

  if (!wait_for(20, ^BOOL {
        NSScreen *a = screen_for(hid), *b = screen_for(lid);
        return a && b && a.backingScaleFactor == 2.0 && b.backingScaleFactor == 1.0 &&
               a.frame.size.width == 1920 && b.frame.size.width == 1280;
      })) {
    for (NSScreen *s in NSScreen.screens)
      fprintf(stderr, "vd: screen id=%u frame=%.0fx%.0f scale=%.1f\n",
              [s.deviceDescription[@"NSScreenNumber"] unsignedIntValue], s.frame.size.width,
              s.frame.size.height, s.backingScaleFactor);
    fprintf(stderr, "vd: wanted 2x id=%u, 1x id=%u\n", hid, lid);
    return fail(stop ? "interrupted" : "NSScreen never listed both displays at scales 2.0 and 1.0");
  }

  for (NSArray *d in @[ @[ @"2x", @(hid) ], @[ @"1x", @(lid) ] ]) {
    CGDirectDisplayID id = [d[1] unsignedIntValue];
    CGRect b = CGDisplayBounds(id);  // global, top-left origin: what System Events positions use
    CGDisplayModeRef m = CGDisplayCopyDisplayMode(id);
    printf("display name=%s id=%u x=%.0f y=%.0f w=%.0f h=%.0f scale=%.1f pixels=%zux%zu\n",
           [d[0] UTF8String], id, b.origin.x, b.origin.y, b.size.width, b.size.height,
           screen_for(id).backingScaleFactor, CGDisplayModeGetPixelWidth(m), CGDisplayModeGetPixelHeight(m));
    CGDisplayModeRelease(m);
  }
  printf("ready\n");

  while (!stop && getppid() == parent) pump(0.2);
  hi = nil; lo = nil;  // tears the displays down
  return 0;
}}
