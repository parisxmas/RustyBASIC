' PWM LED fade example for ESP32-C3 (QBASIC style)
' Fades an LED on pin 2 using PWM channel 0

CONST LED_PIN = 2
CONST PWM_CH = 0
CONST PWM_FREQ = 5000
CONST PWM_RES = 8

PWM.SETUP PWM_CH, LED_PIN, PWM_FREQ, PWM_RES

PRINT "Fading LED..."

DIM duty AS INTEGER

DO
    ' Fade up
    FOR duty = 0 TO 255
        PWM.DUTY PWM_CH, duty
        DELAY 5
    NEXT duty

    ' Fade down
    FOR duty = 255 TO 0 STEP -1
        PWM.DUTY PWM_CH, duty
        DELAY 5
    NEXT duty
LOOP
