' Module example
MODULE Math
    FUNCTION Square(x AS INTEGER) AS INTEGER
        Square = x * x
    END FUNCTION

    FUNCTION Cube(x AS INTEGER) AS INTEGER
        Cube = x * x * x
    END FUNCTION
END MODULE

DIM a AS INTEGER
a = Math.Square(4)
PRINT "4 squared = "; a
a = Math.Cube(3)
PRINT "3 cubed = "; a
END
