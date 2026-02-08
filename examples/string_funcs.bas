' Demonstrates all 12 string built-in functions

DIM s AS STRING
DIM n AS INTEGER

s = "Hello, World!"

' LEN - string length
PRINT "LEN:"; LEN(s)

' LEFT$ - first n characters
PRINT "LEFT$:"; LEFT$(s, 5)

' RIGHT$ - last n characters
PRINT "RIGHT$:"; RIGHT$(s, 6)

' MID$ - substring (1-based start)
PRINT "MID$:"; MID$(s, 8, 5)

' INSTR - find substring position
PRINT "INSTR:"; INSTR(s, "World")

' ASC - ASCII code of first character
PRINT "ASC:"; ASC(s)

' CHR$ - character from ASCII code
PRINT "CHR$:"; CHR$(65)

' STR$ - number to string
PRINT "STR$:"; STR$(3.14)

' VAL - string to number
PRINT "VAL:"; VAL("42.5")

' UCASE$ - uppercase
PRINT "UCASE$:"; UCASE$(s)

' LCASE$ - lowercase
PRINT "LCASE$:"; LCASE$(s)

' TRIM$ - strip whitespace
PRINT "TRIM$:"; TRIM$("  spaced  ")

END
