' SD card example
SD.INIT 5
PRINT "SD card initialized"
SD.OPEN "test.txt", "w"
SD.WRITE "Hello from RustyBASIC!"
SD.CLOSE
PRINT "File written"
SD.OPEN "test.txt", "r"
SD.READ$ content$
SD.CLOSE
PRINT "Read: "; content$
SD.FREE space%
PRINT "Free space: "; space%; " bytes"
