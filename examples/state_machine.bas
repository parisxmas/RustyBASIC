' State machine example
MACHINE TrafficLight
    STATE RED
        ON TIMER GOTO GREEN
    END STATE
    STATE GREEN
        ON TIMER GOTO YELLOW
    END STATE
    STATE YELLOW
        ON TIMER GOTO RED
    END STATE
END MACHINE

PRINT "Traffic light created"
TrafficLight.EVENT "TIMER"
PRINT "After first event"
END
