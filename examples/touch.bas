' Touch Sensor Example
' Read capacitive touch sensor on GPIO 4

DO
    TOUCH.READ 4, value%
    PRINT "Touch value: "; value%
    IF value% < 40 THEN
        PRINT "Touched!"
    END IF
    DELAY 500
LOOP
