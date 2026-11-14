// Virtual HID mouse for the cursor lab (`cargo xtask cursor-lab`).
//   hid
// Tries IOHIDUserDeviceCreateWithProperties for a 3-button relative mouse and prints exactly what
// happened: `create=ok` or `create=fail`, then `ready`. After `ready` it reads commands on stdin:
//   move DX DY N     send N relative-motion reports of (DX, DY), 20 ms apart; prints `sent N`
//   quit             destroy the device and exit
// Stdin EOF also quits. The restricted entitlement com.apple.developer.hid.virtual.device may be
// required (an unsigned or ad-hoc process may be refused or killed by the OS: the caller reports
// the exit status or signal). Nothing here is known to work on hosted runners.
#import <Cocoa/Cocoa.h>
#import <IOKit/hid/IOHIDKeys.h>

// IOHIDUserDevice.h is not in the macOS SDK; the symbols are in IOKit, declared by hand (as vd.m
// does for CGVirtualDisplay). A macOS update can change them.
typedef struct __IOHIDUserDevice *IOHIDUserDeviceRef;
extern IOHIDUserDeviceRef IOHIDUserDeviceCreateWithProperties(CFAllocatorRef allocator, CFDictionaryRef properties, uint32_t options);
extern IOReturn IOHIDUserDeviceHandleReport(IOHIDUserDeviceRef device, const uint8_t *report, CFIndex length);

// Boot-protocol style mouse: 3 buttons, relative X and Y (int8). Report = [buttons, dx, dy].
static const uint8_t kDescriptor[] = {
    0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0xA1, 0x00, 0x05, 0x09, 0x19, 0x01, 0x29, 0x03,
    0x15, 0x00, 0x25, 0x01, 0x95, 0x03, 0x75, 0x01, 0x81, 0x02, 0x95, 0x01, 0x75, 0x05, 0x81, 0x03,
    0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x15, 0x81, 0x25, 0x7F, 0x75, 0x08, 0x95, 0x02, 0x81, 0x06,
    0xC0, 0xC0};

int main(void) {
  NSDictionary *props = @{
    @kIOHIDReportDescriptorKey : [NSData dataWithBytes:kDescriptor length:sizeof kDescriptor],
    @kIOHIDVendorIDKey : @(0x1209),
    @kIOHIDProductIDKey : @(0x0001),
    @kIOHIDProductKey : @"burl cursor lab mouse",
    @kIOHIDTransportKey : @"Virtual",
    @kIOHIDPrimaryUsagePageKey : @(1),
    @kIOHIDPrimaryUsageKey : @(2),
  };
  IOHIDUserDeviceRef dev = IOHIDUserDeviceCreateWithProperties(kCFAllocatorDefault, (__bridge CFDictionaryRef)props, 0);
  if (!dev) {
    printf("create=fail\n");
    printf("ready\n");
    fflush(stdout);
    return 3;
  }
  printf("create=ok\nready\n");
  fflush(stdout);
  char line[128];
  while (fgets(line, sizeof line, stdin)) {
    int dx, dy, count;
    if (sscanf(line, "move %d %d %d", &dx, &dy, &count) == 3) {
      for (int i = 0; i < count; i++) {
        uint8_t report[3] = {0, (uint8_t)(int8_t)dx, (uint8_t)(int8_t)dy};
        IOReturn r = IOHIDUserDeviceHandleReport(dev, report, sizeof report);
        if (r != kIOReturnSuccess) {
          printf("report-error 0x%x\n", (unsigned)r);
          fflush(stdout);
          break;
        }
        usleep(20000);
      }
      printf("sent %d\n", count);
      fflush(stdout);
    } else if (!strncmp(line, "quit", 4)) {
      break;
    }
  }
  CFRelease(dev);
  return 0;
}
