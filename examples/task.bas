' Task example (FreeRTOS on ESP32, pthreads on host)
PRINT "Main task starting"

TASK "blinker", 2048, 1
    PRINT "Blinker task running"
    DELAY 1000
    PRINT "Blinker task done"
END TASK

PRINT "Main task continues"
DELAY 2000
PRINT "Done"
END
