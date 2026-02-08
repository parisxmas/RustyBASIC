' Math built-in functions demo

DIM x AS SINGLE
DIM n AS INTEGER

' Square root
x = SQR(25.0)
PRINT "SQR(25) ="; x

' Absolute value
x = ABS(-7.5)
PRINT "ABS(-7.5) ="; x

' Trigonometry (radians)
x = SIN(1.5708)
PRINT "SIN(pi/2) ="; x

x = COS(0.0)
PRINT "COS(0) ="; x

x = TAN(0.7854)
PRINT "TAN(pi/4) ="; x

x = ATN(1.0)
PRINT "ATN(1) ="; x

' Logarithm and exponential
x = LOG(2.71828)
PRINT "LOG(e) ="; x

x = EXP(1.0)
PRINT "EXP(1) ="; x

' INT -- floor toward -infinity
n = INT(3.7)
PRINT "INT(3.7) ="; n

n = INT(-3.2)
PRINT "INT(-3.2) ="; n

' FIX -- truncate toward zero
n = FIX(3.7)
PRINT "FIX(3.7) ="; n

n = FIX(-3.2)
PRINT "FIX(-3.2) ="; n

' SGN -- sign function
n = SGN(42.0)
PRINT "SGN(42) ="; n

n = SGN(-5.0)
PRINT "SGN(-5) ="; n

n = SGN(0.0)
PRINT "SGN(0) ="; n

' RND -- random float in [0, 1)
x = RND
PRINT "RND ="; x

END
