' STRING$ and SPACE$ Example
' Classic BASIC string functions

DIM s$
s$ = STRING$(10, 42)
PRINT "Stars: "; s$

DIM sp$
sp$ = SPACE$(5)
PRINT "Hello"; sp$; "World"

' Build a simple box
DIM border$
border$ = STRING$(20, 45)
PRINT border$
PRINT "  RustyBASIC  "
PRINT border$
