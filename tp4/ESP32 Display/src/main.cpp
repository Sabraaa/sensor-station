#include <Arduino_GFX_Library.h>

#define LCD_BL 22

Arduino_DataBus *bus = new Arduino_ESP32SPI(15, 14, 7, 6, GFX_NOT_DEFINED);
Arduino_GFX *gfx = new Arduino_ST7789(bus, 21, 0, true, 172, 320, 34, 0, 34, 0);

const uint16_t BG    = 0x0000;
const uint16_t DIM   = 0x8410;   // grey
const uint16_t HILITE= 0x07FF;   // cyan
const uint16_t TEXT  = 0xFFFF;

// row tops
const int ROW[3] = {60, 150, 240};

void drawChrome() {
  gfx->fillScreen(BG);

  gfx->setTextColor(HILITE);
  gfx->setTextSize(2);
  gfx->setCursor(10, 15);
  gfx->print("SENSOR");
  gfx->drawFastHLine(10, 42, 152, DIM);

  const char *labels[3] = {"TEMPERATUR", "HOEHE", "DRUCK"};
  gfx->setTextSize(1);
  gfx->setTextColor(DIM);
  for (int i = 0; i < 3; i++) {
    gfx->setCursor(12, ROW[i]);
    gfx->print(labels[i]);
  }
}



void drawValue(int row, const char *value, const char *unit) {
  // erase just this value's box
  gfx->fillRect(10, ROW[row] + 14, 152, 26, BG);

  gfx->setTextColor(TEXT);
  gfx->setTextSize(3);
  gfx->setCursor(12, ROW[row] + 16);
  gfx->print(value);

  gfx->setTextSize(1);
  gfx->setTextColor(DIM);
  gfx->print(" ");
  gfx->print(unit);
}

void setup() {
  pinMode(LCD_BL, OUTPUT);
  digitalWrite(LCD_BL, HIGH);
  gfx->begin();
  drawChrome();
}

void loop() {
  // placeholder values until a sensor is wired up
  static float t = 21.4, h = 142.0, p = 1013.0;
  char buf[16];

  snprintf(buf, sizeof(buf), "%.1f", t);  drawValue(0, buf, "C");
  snprintf(buf, sizeof(buf), "%.0f", h);  drawValue(1, buf, "m");
  snprintf(buf, sizeof(buf), "%.0f", p);  drawValue(2, buf, "hPa");

  t += 0;
  delay(1000);
}