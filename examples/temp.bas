' Temperature Sensor Example
' Read internal temperature sensor

DO
    TEMP.READ t
    PRINT "Temperature: "; t; " C"
    DELAY 2000
LOOP
