' ADC analog input example for ESP32-C3 (QBASIC style)
' Reads an analog sensor on ADC channel 0

DIM reading AS INTEGER

PRINT "Reading ADC..."

DO
    ADC.READ 0, reading
    PRINT "ADC value: "; reading
    DELAY 1000
LOOP
