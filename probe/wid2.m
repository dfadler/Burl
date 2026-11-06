#import <Cocoa/Cocoa.h>
int main(int argc,char**argv){ NSString*o=[NSString stringWithUTF8String:argv[1]]; NSArray *a=CFBridgingRelease(CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly,kCGNullWindowID));
 for(NSDictionary*w in a) if([w[@"kCGWindowOwnerName"] isEqual:o]){ CGRect r; CGRectMakeWithDictionaryRepresentation((__bridge CFDictionaryRef)w[@"kCGWindowBounds"],&r); if(r.size.height>100) printf("%d %.0f %.0f %.0f %.0f\n",[w[@"kCGWindowNumber"] intValue],r.origin.x,r.origin.y,r.size.width,r.size.height);} return 0;}
