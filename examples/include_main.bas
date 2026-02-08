' Demonstrates the INCLUDE directive
INCLUDE "include_lib.bas"

DIM r AS SINGLE
r = 5.0

CALL PrintBanner(GREETING)
PRINT "Radius:"; r
PRINT "Area:"; CircleArea!(r)

END
