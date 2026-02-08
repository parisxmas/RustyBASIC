' Timer stopwatch example (QBASIC style)
' Measures elapsed time in milliseconds

DIM elapsed AS INTEGER

TIMER.START

PRINT "Working..."
DELAY 1500

TIMER.ELAPSED elapsed
PRINT "Elapsed time: "; elapsed; " ms"

END
