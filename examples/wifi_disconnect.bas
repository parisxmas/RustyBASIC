' WiFi connect and disconnect example for ESP32-C3 (QBASIC style)
' Demonstrates the full WiFi lifecycle

DIM ssid AS STRING
DIM pass AS STRING
DIM status AS INTEGER

ssid = "MyNetwork"
pass = "MyPassword"

PRINT "Connecting to WiFi..."
WIFI.CONNECT ssid, pass
DELAY 3000

WIFI.STATUS status
IF status = 1 THEN
    PRINT "Connected!"

    PRINT "Doing some work..."
    DELAY 2000

    PRINT "Disconnecting..."
    WIFI.DISCONNECT
    DELAY 1000

    WIFI.STATUS status
    IF status = 0 THEN
        PRINT "Disconnected successfully."
    ELSE
        PRINT "Still connected."
    END IF
ELSE
    PRINT "Failed to connect."
END IF

END
