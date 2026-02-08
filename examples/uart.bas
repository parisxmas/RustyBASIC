' UART serial communication example for ESP32-C3 (QBASIC style)
' Sets up UART port 1 and echoes received bytes

CONST PORT = 1
CONST BAUD = 9600
CONST TX_PIN = 4
CONST RX_PIN = 5

DIM received AS INTEGER

UART.SETUP PORT, BAUD, TX_PIN, RX_PIN
PRINT "UART ready. Echoing data..."

DO
    UART.READ PORT, received
    IF received >= 0 THEN
        UART.WRITE PORT, received
        PRINT "Echo: "; received
    END IF
    DELAY 10
LOOP
