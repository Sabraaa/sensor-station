#include <Wire.h>
#include <SoftwareSerial.h>
#include <Adafruit_Sensor.h>
#include <Adafruit_BME280.h>
#include <MHZ19.h>
 
// Pins für den MH-Z19C CO2-Sensor
#define MHZ_RX_PIN 10
#define MHZ_TX_PIN 11
float no2 = 89.0;
float nh3 = 89.0;
float co = 89.0 ;
MHZ19 myMHZ19;
SoftwareSerial mhzSerial(MHZ_RX_PIN, MHZ_TX_PIN);
Adafruit_BME280 bme; // I2C-Instanz für BME280
 
unsigned long measurementTimer = 0;
 
void setup() {
  Serial.begin(9600);
  while (!Serial); // Warten auf Seriellen Monitor
 
  //Serial.println(F("=== Kombi-Sensor-Station gestartet ==="));
 
  // 1. Initialisiere MH-Z19C (UART)
  mhzSerial.begin(9600);
  myMHZ19.begin(mhzSerial);
  myMHZ19.autoCalibration(false); // Automatische Kalibrierung aus
 
  // 2. Initialisiere BME280 (I2C)
  if (!bme.begin(0x76)) {
    Serial.println(F("Kritischer Fehler: BME280 nicht gefunden! Verkabelung prüfen."));
    while (1); // Programm anhalten
  }
 
  //Serial.println(F("Beide Sensoren erfolgreich initialisiert. Warte auf erste Messung..."));
  //Serial.println(F("--------------------------------------------------"));
  delay(2000);
}
 
void loop() {
  // Alle 5 Sekunden (5000 ms) Daten auslesen
  if (millis() - measurementTimer >= 5000) {
   
    // Daten vom MH-Z19C holen
    int co2 = myMHZ19.getCO2();
   
    // Daten vom BME280 holen
    float tempBME = bme.readTemperature();
    float humidity = bme.readHumidity();
    float pressure = bme.readPressure() / 100.0F; 
 
    // Ausgabe im Seriellen Monitor
   /* Serial.print(F("CO2-Gehalt:   "));
    if (co2 > 0) {
      Serial.print(co2);
      Serial.println(F(" ppm"));
    } else {
      Serial.println(F("Fehler beim Lesen des MH-Z19C!"));
    }
 
    Serial.print(F("Temperatur:   "));
    Serial.print(tempBME, 1); // 1 Nachkommastelle
    Serial.println(F(" °C"));
 
    Serial.print(F("Luftfeuchte:  "));
    Serial.print(humidity, 1);
    Serial.println(F(" %"));
 
    Serial.print(F("Luftdruck:    "));
    Serial.print(pressure, 1);
    Serial.println(F(" hPa"));
 
    Serial.println(F("--------------------------------------------------"));
*/
    // Kompakter CSV-String: co2;temperatur;luftfeuchte;luftdruck
    String csv = String(co2) + ";" + String(tempBME, 1) + ";" +
                 String(humidity, 1) + ";" + String(pressure, 1);
    Serial.println(csv); // z.B. "650;23.2;45.4;1021.1"
 
    measurementTimer = millis(); 
  }
}