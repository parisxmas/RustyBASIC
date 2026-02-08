' I2C communication example for ESP32-C3 (QBASIC style)
' Reads temperature from a BMP280 sensor over I2C

CONST I2C_BUS = 0
CONST SDA_PIN = 4
CONST SCL_PIN = 5
CONST I2C_FREQ = 100000

CONST BMP280_ADDR = 118
CONST REG_CHIP_ID = 208
CONST REG_TEMP_MSB = 250

DIM chipId AS INTEGER
DIM rawTemp AS INTEGER

PRINT "Initializing I2C bus..."
I2C.SETUP I2C_BUS, SDA_PIN, SCL_PIN, I2C_FREQ

' Write register address, then read chip ID
I2C.WRITE BMP280_ADDR, REG_CHIP_ID
I2C.READ BMP280_ADDR, 1, chipId
PRINT "Chip ID:"; chipId

IF chipId = 88 THEN
    PRINT "BMP280 detected!"

    ' Read raw temperature MSB
    I2C.WRITE BMP280_ADDR, REG_TEMP_MSB
    I2C.READ BMP280_ADDR, 1, rawTemp
    PRINT "Raw temp MSB:"; rawTemp
ELSE
    PRINT "Unknown device"
END IF

END
