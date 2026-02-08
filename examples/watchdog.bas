' Watchdog timer example
WDT.ENABLE 5000
PRINT "Watchdog enabled (5s timeout)"
FOR i = 1 TO 10
    PRINT "Working... "; i
    WDT.FEED
    DELAY 1000
NEXT i
WDT.DISABLE
PRINT "Watchdog disabled"
