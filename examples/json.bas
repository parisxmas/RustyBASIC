' JSON parsing demo
DIM json$ AS STRING
DIM result$ AS STRING
DIM count AS INTEGER
DIM updated$ AS STRING

' Build JSON via JSON.SET
DIM base$ AS STRING
base$ = "{}"
JSON.SET base$, "name", "RustyBASIC", json$
JSON.SET json$, "version", "2", json$

JSON.GET json$, "name", result$
PRINT "Name: "; result$

JSON.GET json$, "version", result$
PRINT "Version: "; result$

JSON.COUNT json$, count
PRINT "Key count: "; count

JSON.SET json$, "author", "ESP32", updated$
PRINT "Updated: "; updated$

END
