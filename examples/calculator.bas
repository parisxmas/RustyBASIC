' Simple calculator using SUB/FUNCTION (QBASIC style)

DECLARE SUB ShowResult (label AS STRING, value AS SINGLE)
DECLARE FUNCTION Add! (a AS SINGLE, b AS SINGLE)

DIM a AS SINGLE
DIM b AS SINGLE

PRINT "Simple Calculator"
INPUT "First number: "; a
INPUT "Second number: "; b

CALL ShowResult("A + B", Add!(a, b))
CALL ShowResult("A - B", a - b)
CALL ShowResult("A * B", a * b)

IF b <> 0 THEN
    CALL ShowResult("A / B", a / b)
ELSE
    PRINT "Cannot divide by zero!"
END IF

END

SUB ShowResult (label AS STRING, value AS SINGLE)
    PRINT label; " = "; value
END SUB

FUNCTION Add! (a AS SINGLE, b AS SINGLE)
    Add! = a + b
END FUNCTION
