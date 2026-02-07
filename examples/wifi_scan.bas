' WiFi connection example for ESP32-C3 (QBASIC style)

DIM ssid AS STRING
DIM pass AS STRING
DIM status AS INTEGER

ssid = "MyNetwork"
pass = "MyPassword"

PRINT "Connecting to WiFi..."
WIFI.CONNECT ssid, pass
DELAY 3000

WIFI.STATUS status
SELECT CASE status
    CASE 1
        PRINT "Connected!"
    CASE 0
        PRINT "Failed to connect."
    CASE ELSE
        PRINT "Unknown status"
END SELECT

END
