// Gnarl HID rig firmware: a Raspberry Pi Pico (RP2040) as a USB HID mouse, driven by text lines
// over its USB serial port. Protocol (one command per line, replies `ok [info]` or `err MSG`):
//   ping            ->  ok gnarl-hid-rig 1
//   rel DX DY       ->  ok          relative motion, each part -127..127, no wheel
//   down | up       ->  ok          left button press / release
// Build: Arduino IDE or arduino-cli with the arduino-pico core (earlephilhower), board "Raspberry
// Pi Pico", Tools > USB Stack = "Pico SDK" (the Adafruit TinyUSB stack has no Mouse library).
// The host side is xtask/src/hid_rig.rs; `parse` there is the spec of this parser.
#include <Mouse.h>

static const unsigned long HOLD_TIMEOUT_MS = 5000;  // safety: a button held this long with no command is released
static char buf[32];
static size_t n = 0;
static bool overflow = false;
static bool held = false;
static unsigned long last_cmd = 0;

static void handle(const char *line) {
  int dx, dy;
  char extra;
  if (!strcmp(line, "ping")) {
    Serial.println("ok gnarl-hid-rig 1");
  } else if (!strcmp(line, "down")) {
    Mouse.press(MOUSE_LEFT);
    held = true;
    Serial.println("ok");
  } else if (!strcmp(line, "up")) {
    Mouse.release(MOUSE_LEFT);
    held = false;
    Serial.println("ok");
  } else if (sscanf(line, "rel %d %d%c", &dx, &dy, &extra) == 2 && dx >= -127 && dx <= 127 && dy >= -127 && dy <= 127) {
    Mouse.move(dx, dy, 0);
    Serial.println("ok");
  } else {
    Serial.println("err bad command");
  }
}

void setup() {
  Serial.begin(115200);
  Mouse.begin();
}

void loop() {
  while (Serial.available()) {
    char c = Serial.read();
    last_cmd = millis();
    if (c == '\r') continue;
    if (c == '\n') {
      buf[n] = 0;
      if (overflow) Serial.println("err line too long");
      else handle(buf);
      n = 0;
      overflow = false;
    } else if (n < sizeof buf - 1) {
      buf[n++] = c;
    } else {
      overflow = true;
    }
  }
  if (held && millis() - last_cmd > HOLD_TIMEOUT_MS) {
    Mouse.release(MOUSE_LEFT);
    held = false;
  }
}
