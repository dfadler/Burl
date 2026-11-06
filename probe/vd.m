#import <Cocoa/Cocoa.h>
// Private CoreGraphics API, declared by hand.
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

static void dump(const char *tag) {
  printf("-- %s\n", tag);
  for (NSScreen *s in NSScreen.screens) {
    NSNumber *id_ = s.deviceDescription[@"NSScreenNumber"];
    CGDirectDisplayID d = id_.unsignedIntValue;
    CGDisplayModeRef m = CGDisplayCopyDisplayMode(d);
    printf("id=%u frame=%.0fx%.0f@(%.0f,%.0f) backingScale=%.1f mode=%zux%zu pixels=%zux%zu\n", d,
      s.frame.size.width, s.frame.size.height, s.frame.origin.x, s.frame.origin.y, s.backingScaleFactor,
      CGDisplayModeGetWidth(m), CGDisplayModeGetHeight(m), CGDisplayModeGetPixelWidth(m), CGDisplayModeGetPixelHeight(m));
    CGDisplayModeRelease(m);
  }
}
static void pump(double sec){ NSDate *e=[NSDate dateWithTimeIntervalSinceNow:sec]; while([e timeIntervalSinceNow]>0){ NSEvent *ev=[NSApp nextEventMatchingMask:NSEventMaskAny untilDate:[NSDate dateWithTimeIntervalSinceNow:0.1] inMode:NSDefaultRunLoopMode dequeue:YES]; if(ev)[NSApp sendEvent:ev]; } }
int main(int argc, char**argv) { @autoreleasepool {
  setvbuf(stdout,NULL,_IONBF,0); [NSApplication sharedApplication]; [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory]; [NSApp finishLaunching];
  dump("before");
  CGVirtualDisplayDescriptor *desc = [CGVirtualDisplayDescriptor new];
  desc.name = @"burl-e1"; desc.vendorID = 0x1234; desc.productID = 0x5678; desc.serialNum = 1;
  desc.maxPixelsWide = 3840; desc.maxPixelsHigh = 2160;
  desc.sizeInMillimeters = CGSizeMake(600, 340);
  desc.queue = dispatch_get_main_queue();
  CGVirtualDisplay *vd = [[CGVirtualDisplay alloc] initWithDescriptor:desc];
  if (!vd) { printf("create failed\n"); return 1; }
  CGVirtualDisplaySettings *set = [CGVirtualDisplaySettings new];
  set.hiDPI = 1;
  set.modes = @[ [[CGVirtualDisplayMode alloc] initWithWidth:3840 height:2160 refreshRate:60],
                 [[CGVirtualDisplayMode alloc] initWithWidth:1920 height:1080 refreshRate:60] ];
  printf("applySettings=%d displayID=%u\n", [vd applySettings:set], vd.displayID);
  pump(3);
  dump("after create");
  // list modes of the virtual display, incl. HiDPI flag
  NSDictionary *o = @{(id)kCGDisplayShowDuplicateLowResolutionModes: @YES};
  { CGDirectDisplayID l[16]; uint32_t n=0; CGGetOnlineDisplayList(16,l,&n); printf("online:"); for(uint32_t i=0;i<n;i++) printf(" %u(active=%d)",l[i],CGDisplayIsActive(l[i])); printf("\n"); }
  CFArrayRef modes = CGDisplayCopyAllDisplayModes(vd.displayID, (__bridge CFDictionaryRef)o); if(!modes) modes = CGDisplayCopyAllDisplayModes(vd.displayID, NULL);
  CGDisplayModeRef want = NULL; if (!modes) { printf("no modes for vd\n"); return 2; }
  for (CFIndex i = 0; i < CFArrayGetCount(modes); i++) {
    CGDisplayModeRef m = (CGDisplayModeRef)CFArrayGetValueAtIndex(modes, i);
    printf("mode %zux%zu px=%zux%zu\n", CGDisplayModeGetWidth(m), CGDisplayModeGetHeight(m), CGDisplayModeGetPixelWidth(m), CGDisplayModeGetPixelHeight(m));
    if (CGDisplayModeGetPixelWidth(m) == 2*CGDisplayModeGetWidth(m) && CGDisplayModeGetWidth(m) == 1920) want = m;
  }
  if (want) {
    CGDisplayConfigRef cfg; CGBeginDisplayConfiguration(&cfg);
    CGConfigureDisplayWithDisplayMode(cfg, vd.displayID, want, NULL);
    printf("complete=%d\n", CGCompleteDisplayConfiguration(cfg, kCGConfigureForAppOnly));
    pump(3);
    dump("after HiDPI mode switch");
  } else printf("no 1920x1080@2x mode found\n");
  // E2: second display, 1x, 1280x720
  CGVirtualDisplayDescriptor *d2 = [CGVirtualDisplayDescriptor new];
  d2.name=@"burl-e2"; d2.vendorID=0x1234; d2.productID=0x5679; d2.serialNum=2;
  d2.maxPixelsWide=1280; d2.maxPixelsHigh=720; d2.sizeInMillimeters=CGSizeMake(300,170); d2.queue=dispatch_get_main_queue();
  CGVirtualDisplay *v2=[[CGVirtualDisplay alloc] initWithDescriptor:d2];
  CGVirtualDisplaySettings *s2=[CGVirtualDisplaySettings new]; s2.hiDPI=0;
  s2.modes=@[[[CGVirtualDisplayMode alloc] initWithWidth:1280 height:720 refreshRate:60]];
  printf("v2 applySettings=%d id=%u\n",[v2 applySettings:s2],v2.displayID);
  pump(3);
  dump("after second display");
  pump(argc>1?atof(argv[1]):10);
  return 0;
}}
