' Enum example
ENUM Color
    RED
    GREEN = 10
    BLUE
END ENUM

DIM c AS INTEGER
c = Color.GREEN
PRINT "Green = "; c
IF c = Color.GREEN THEN PRINT "It's green!"
END
