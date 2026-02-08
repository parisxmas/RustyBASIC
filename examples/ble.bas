' BLE echo server example for ESP32-C3
DIM msg$ AS STRING

BLE.INIT "RustyBASIC-BLE"
BLE.ADVERTISE 1

PRINT "BLE advertising started..."

DO
    BLE.RECEIVE msg$
    IF msg$ <> "" THEN
        PRINT "Received: "; msg$
        BLE.SEND msg$
    END IF
    DELAY 100
LOOP

END
