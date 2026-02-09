' Bitwise shift operators example
DIM a AS INTEGER
a = 1
PRINT "1 SHL 4 = "; a SHL 4
PRINT "16 SHR 2 = "; 16 SHR 2
DIM flags AS INTEGER
flags = 1 SHL 0 OR 1 SHL 2 OR 1 SHL 4
PRINT "Flags: "; flags
PRINT "Bit 2 set: "; (flags SHR 2) AND 1
