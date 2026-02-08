' SPI communication example for ESP32-C3 (QBASIC style)
' Reads the WHO_AM_I register from an SPI sensor (e.g. BME280)

CONST SPI_BUS = 2
CONST CLK_PIN = 6
CONST MOSI_PIN = 7
CONST MISO_PIN = 2
CONST SPI_FREQ = 1000000

CONST WHO_AM_I = 208

DIM response AS INTEGER

PRINT "Initializing SPI bus..."
SPI.SETUP SPI_BUS, CLK_PIN, MOSI_PIN, MISO_PIN, SPI_FREQ

' Read WHO_AM_I register (0xD0 with read bit set = 0xD0)
PRINT "Reading WHO_AM_I register..."
SPI.TRANSFER 208, response
PRINT "Device ID:"; response

IF response = 96 THEN
    PRINT "BME280 detected!"
ELSEIF response = 88 THEN
    PRINT "BMP280 detected!"
ELSE
    PRINT "Unknown device"
END IF

END
