' FOR EACH example
DIM scores(4) AS INTEGER
scores(0) = 90
scores(1) = 85
scores(2) = 92
scores(3) = 78
scores(4) = 95

DIM total AS INTEGER
total = 0
FOR EACH s IN scores
    total = total + s
    PRINT "Score: "; s
NEXT

PRINT "Total: "; total
END
