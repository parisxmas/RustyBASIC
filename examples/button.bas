' Button input example for ESP32-C3 (QBASIC style)
' Reads a push button and toggles an LED

CONST BUTTON_PIN = 9
CONST LED_PIN = 2
CONST INPUT_MODE = 0
CONST OUTPUT_MODE = 1

DIM state AS INTEGER
DIM ledOn AS INTEGER

GPIO.MODE BUTTON_PIN, INPUT_MODE
GPIO.MODE LED_PIN, OUTPUT_MODE

ledOn = 0
GPIO.SET LED_PIN, 0

PRINT "Press the button to toggle LED..."

DO
    GPIO.READ BUTTON_PIN, state

    IF state = 0 THEN
        ' Button pressed (active low)
        IF ledOn = 0 THEN
            ledOn = 1
            GPIO.SET LED_PIN, 1
            PRINT "LED ON"
        ELSE
            ledOn = 0
            GPIO.SET LED_PIN, 0
            PRINT "LED OFF"
        END IF

        ' Simple debounce
        DELAY 300
    END IF

    DELAY 50
LOOP
