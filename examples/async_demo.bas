' Async / cooperative multitasking example
PRINT "Starting async demo"
FOR i = 1 TO 5
    PRINT "Working... "; i
    YIELD
    AWAIT 500
NEXT i
PRINT "Done"
