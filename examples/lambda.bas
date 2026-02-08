' Lambda example
DIM square AS FUNCTION
square = LAMBDA(x AS INTEGER) => x * x

DIM result AS INTEGER
result = square(5)
PRINT "5 squared = "; result
END
