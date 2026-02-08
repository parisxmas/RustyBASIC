' HTTP client example for ESP32-C3 (QBASIC style)
' Requires WiFi to be connected first

DIM ssid AS STRING
DIM pass AS STRING
DIM status AS INTEGER
DIM response$ AS STRING

ssid = "MyNetwork"
pass = "MyPassword"

PRINT "Connecting to WiFi..."
WIFI.CONNECT ssid, pass
DELAY 3000

WIFI.STATUS status
IF status = 1 THEN
    PRINT "Connected! Making HTTP GET request..."
    HTTP.GET "http://httpbin.org/get", response$
    PRINT "Response: "; response$
ELSE
    PRINT "WiFi connection failed."
END IF

END
