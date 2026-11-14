/* A virtual relative mouse made with the kernel's uinput, so Xorg's libinput driver reads it as a real
 * evdev pointer (scripts/window-uinput.sh). Needs write access to /dev/uinput.
 * Creates the device, prints "ready <name>" on stdout, then reads one command per line on stdin:
 *   rel DX DY      one relative motion event
 *   down|up        right button (BTN_RIGHT)
 *   quit           destroy the device and exit
 * EOF on stdin quits too. Exit 2 when the device cannot be created. */
#include <fcntl.h>
#include <linux/uinput.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

static int fd;

static void emit(int type, int code, int val) {
	struct input_event ev;
	memset(&ev, 0, sizeof ev);
	ev.type = type;
	ev.code = code;
	ev.value = val;
	if (write(fd, &ev, sizeof ev) != sizeof ev)
		perror("uinput-mouse: write");
}

int main(void) {
	fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK);
	if (fd < 0) {
		perror("uinput-mouse: open /dev/uinput");
		return 2;
	}
	ioctl(fd, UI_SET_EVBIT, EV_KEY);
	ioctl(fd, UI_SET_KEYBIT, BTN_LEFT);
	ioctl(fd, UI_SET_KEYBIT, BTN_RIGHT);
	ioctl(fd, UI_SET_EVBIT, EV_REL);
	ioctl(fd, UI_SET_RELBIT, REL_X);
	ioctl(fd, UI_SET_RELBIT, REL_Y);
	struct uinput_setup us;
	memset(&us, 0, sizeof us);
	us.id.bustype = BUS_USB;
	us.id.vendor = 0x1234;
	us.id.product = 0x5678;
	strcpy(us.name, "gnarl-uinput-mouse");
	if (ioctl(fd, UI_DEV_SETUP, &us) < 0 || ioctl(fd, UI_DEV_CREATE) < 0) {
		perror("uinput-mouse: create device");
		return 2;
	}
	printf("ready %s\n", us.name);
	fflush(stdout);

	char line[128];
	int dx, dy;
	while (fgets(line, sizeof line, stdin)) {
		if (sscanf(line, "rel %d %d", &dx, &dy) == 2) {
			emit(EV_REL, REL_X, dx);
			emit(EV_REL, REL_Y, dy);
			emit(EV_SYN, SYN_REPORT, 0);
		} else if (!strncmp(line, "down", 4) || !strncmp(line, "up", 2)) {
			emit(EV_KEY, BTN_RIGHT, line[0] == 'd');
			emit(EV_SYN, SYN_REPORT, 0);
		} else if (!strncmp(line, "quit", 4)) {
			break;
		}
	}
	ioctl(fd, UI_DEV_DESTROY);
	close(fd);
	return 0;
}
