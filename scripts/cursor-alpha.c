/* Prints the largest alpha (0-255) in the cursor image the X server currently displays, read with
 * XFixesGetCursorImage. winit hides the pointer with a 1x1 empty-mask cursor set on the window, so
 * the image is fully transparent (0) while it applies and has nonzero alpha otherwise. Only
 * meaningful with the pointer over the window whose cursor is being judged.
 * Exit 0 with the number on stdout; exit 2 when the display or the image is unavailable.
 * Built in docker/window.Dockerfile and used by scripts/window-in-container.sh. */
#include <X11/Xlib.h>
#include <X11/extensions/Xfixes.h>
#include <stdio.h>

int main(void) {
	Display *dpy = XOpenDisplay(NULL);
	if (!dpy) {
		fprintf(stderr, "cursor-alpha: cannot open display\n");
		return 2;
	}
	XFixesCursorImage *img = XFixesGetCursorImage(dpy);
	if (!img) {
		fprintf(stderr, "cursor-alpha: XFixesGetCursorImage failed\n");
		XCloseDisplay(dpy);
		return 2;
	}
	unsigned long max = 0;
	for (unsigned long i = 0; i < (unsigned long)img->width * img->height; i++) {
		unsigned long a = (img->pixels[i] >> 24) & 0xff;
		if (a > max)
			max = a;
	}
	printf("%lu\n", max);
	XFree(img);
	XCloseDisplay(dpy);
	return 0;
}
