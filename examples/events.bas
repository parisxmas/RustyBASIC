' Event system example
DIM count AS INTEGER
count = 0

ON TIMER 1000 GOSUB tick
PRINT "Timer event registered"
DELAY 5000
PRINT "Count: "; count
END

tick:
    count = count + 1
    PRINT "Tick!"
RETURN
