' Shared constants and helpers for include_main.bas

CONST PI = 3.14159
CONST GREETING = "Hello from the library!"

DECLARE SUB PrintBanner (title AS STRING)
DECLARE FUNCTION CircleArea! (radius AS SINGLE)

SUB PrintBanner (title AS STRING)
    PRINT "==========================="
    PRINT " "; title
    PRINT "==========================="
END SUB

FUNCTION CircleArea! (radius AS SINGLE)
    CircleArea! = PI * radius * radius
END FUNCTION
