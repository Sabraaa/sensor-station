#include <Wire.h>
#include <SoftwareSerial.h>
#include <Adafruit_Sensor.h>
#include <Adafruit_BME280.h>
#include <MHZ19.h>
#include "DFRobot_MICS.h"
 
// Pins für den MH-Z19C CO2-Sensor
#define MHZ_RX_PIN 10
#define MHZ_TX_PIN 11
// Pin für den MiCS-5524 Rohgas-Sensor
#define GAS_PIN A0
#define MICS_POWER_PIN 4      // EN-Pin (bei dir fest an 5V, Pin bleibt unbenutzt)
#define CALIBRATION_TIME 3    // Aufwärmzeit in Minuten
#define GAS_MITTELWERT_ANZAHL 16

// Messintervall (MH-Z19C liefert ca. 1 Wert pro Sekunde)
const unsigned long MESSINTERVALL = 1000UL;

float no2 = 0;                // MiCS-5524 kann kein NO2 messen
float nh3 = 0;
float co = 0;
MHZ19 myMHZ19;
SoftwareSerial mhzSerial(MHZ_RX_PIN, MHZ_TX_PIN);
Adafruit_BME280 bme; // I2C-Instanz für BME280
DFRobot_MICS_ADC mics(GAS_PIN, MICS_POWER_PIN);
 
unsigned long measurementTimer = 0;
 
void setup() {
  Serial.begin(9600);
  while (!Serial); // Warten auf Seriellen Monitor
 
  // 1. Initialisiere MH-Z19C (UART)
  mhzSerial.begin(9600);
  myMHZ19.begin(mhzSerial);
  myMHZ19.autoCalibration(false); // Automatische Kalibrierung aus
 
  // 2. Initialisiere BME280 (I2C)
  if (!bme.begin(0x76)) {
    Serial.println(F("Kritischer Fehler: BME280 nicht gefunden! Verkabelung prüfen."));
    while (1); // Programm anhalten
  }

  // 3. Initialisiere MiCS-5524 und warte auf Aufwärmzeit (keine Ausgabe)
  mics.begin();
  if (mics.getPowerState() == SLEEP_MODE) {
    mics.wakeUpMode();
  }
  while (!mics.warmUpTime(CALIBRATION_TIME)) {
    delay(1000);
  }
 
  delay(2000);
}
 
void loop() {
  // Jede Sekunde Daten auslesen
  if (millis() - measurementTimer >= MESSINTERVALL) {
   
    // Daten vom MH-Z19C holen
    int co2 = myMHZ19.getCO2();
   
    // Daten vom BME280 holen
    float tempBME = bme.readTemperature();
    float humidity = bme.readHumidity();
    float pressure = bme.readPressure() / 100.0F; 

    // Rohwert vom MiCS-5524: Mittelwert aus mehreren Messungen (weniger Rauschen)
    long summe = 0;
    for (int i = 0; i < GAS_MITTELWERT_ANZAHL; i++) {
      summe += analogRead(GAS_PIN);
      delay(2);
    }
    int gas = (int)(summe / GAS_MITTELWERT_ANZAHL);

    // Geschätzte Werte aus der Datenblatt-Kurve (ppm)
    co  = mics.getGasData(CO);
    nh3 = mics.getGasData(NH3);

    // Kompakter CSV-String: co2;temperatur;luftfeuchte;luftdruck;gas;no2;nh3;co
    String csv = String(co2) + ";" + String(tempBME, 1) + ";" +
                 String(humidity, 1) + ";" + String(pressure, 1) + ";" +
                 String(gas) + ";" + String(no2, 1) + ";" +
                 String(nh3, 1) + ";" + String(co, 1);
    Serial.println(csv); // z.B. "650;23.2;45.4;1021.1;312;0.0;1.2;3.4"
 
    measurementTimer = millis(); 
  }
}