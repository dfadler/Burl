#import <Cocoa/Cocoa.h>
#import <ApplicationServices/ApplicationServices.h>
int main(void){ printf("screen_capture_access=%d ax_trusted=%d\n", CGPreflightScreenCaptureAccess(), AXIsProcessTrusted());
 CGDirectDisplayID l[16]; uint32_t n=0; CGGetOnlineDisplayList(16,l,&n); printf("online_displays=%u\n",n);
 for(uint32_t i=0;i<n;i++){ CGDisplayModeRef m=CGDisplayCopyDisplayMode(l[i]); printf(" id=%u %zux%zu px=%zux%zu\n",l[i],CGDisplayModeGetWidth(m),CGDisplayModeGetHeight(m),CGDisplayModeGetPixelWidth(m),CGDisplayModeGetPixelHeight(m)); }
 return 0; }
