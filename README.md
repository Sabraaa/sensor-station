# sensor-station
Arduino Nano sensor station: BME280 (temperature, humidity, pressure) and MH-Z19C (CO2), CSV output over Serial

## Web-Part
mysql-connector-python
Flask

## Libraries
- Adafruit BME280, Adafruit Unified Sensor
- MHZ19 (by Jonathan Dempsey)
- DFRobot_MICS (for the MiCS-5524): not available in the Library Manager. Download the ZIP from https://github.com/DFRobot/DFRobot_MICS/archive/refs/heads/master.zip and install it in Arduino IDE via **Sketch → Include Library → Add .ZIP Library...**. Restart the IDE afterwards.

## Warm-up time
After upload or power-on, the MiCS-5524 needs about 3 minutes to heat up. During this time the sketch prints nothing over Serial. Start the sensor in clean, ventilated air, because the library measures its baseline during the warm-up. CSV data only appears after the warm-up is finished.

## Serial output
9600 baud, one CSV line every 5 seconds:

`co2;temperature;humidity;pressure;gas;no2;nh3;co`

| Value | Sensor | Unit |
|---|---|---|
| co2 | MH-Z19C | ppm |
| temperature | BME280 | °C |
| humidity | BME280 | % |
| pressure | BME280 | hPa |
| gas | MiCS-5524 | raw 0-1023 |
| no2 | not measurable (always 0.0) | - |
| nh3 | MiCS-5524, estimated | ppm |
| co | MiCS-5524, estimated | ppm |
