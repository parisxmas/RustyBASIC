' DEF FN Example
' User-defined functions

DEF FNsquare(x) = x * x
DEF FNcube(x) = x * x * x
DEF FNhypotenuse(a, b) = SQR(a * a + b * b)

PRINT "5 squared = "; FNsquare(5)
PRINT "3 cubed = "; FNcube(3)
PRINT "Hypotenuse(3,4) = "; FNhypotenuse(3, 4)
