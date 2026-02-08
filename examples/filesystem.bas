' File system (LittleFS) example
FILE.OPEN "test.txt", "w"
FILE.WRITE "Hello from RustyBASIC!"
FILE.CLOSE
FILE.EXISTS "test.txt", found%
PRINT "File exists: "; found%
FILE.OPEN "test.txt", "r"
FILE.READ$ content$
FILE.CLOSE
PRINT "Read: "; content$
FILE.DELETE "test.txt"
