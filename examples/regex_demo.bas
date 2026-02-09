' Regex example
DIM text$ AS STRING
text$ = "Hello World 123"
REGEX.MATCH "[0-9]+", text$, found%
PRINT "Has numbers: "; found%
REGEX.FIND$ "[0-9]+", text$, match$
PRINT "Found: "; match$
REGEX.REPLACE$ "[0-9]+", text$, "456", result$
PRINT "Replaced: "; result$
